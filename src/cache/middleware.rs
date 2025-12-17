//! L1 response cache middleware.
//!
//! Caches GET requests to public routes and serves cached responses.
//! Skips caching for datastar streaming requests (SSE).

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode, header::CONTENT_LENGTH},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tracing::{debug, instrument};

use super::{
    CacheConfig, CacheRegistry, L1Store, deps,
    keys::{CacheKey, L1Key, OutputFormat, hash_query},
    store::CachedResponse,
};

/// Shared cache state for middleware.
#[derive(Clone)]
pub struct CacheState {
    pub config: CacheConfig,
    pub l1: Arc<L1Store>,
    pub registry: Arc<CacheRegistry>,
}

/// Middleware for L1 response caching.
///
/// Only caches GET requests to public routes that return 200 OK, plus
/// tag/month 404 responses to avoid repeated lookups.
/// Uses `deps::with_collector()` to track dependencies for invalidation.
/// Skips caching for datastar streaming requests.
#[instrument(skip_all, fields(path = %request.uri().path()))]
pub async fn response_cache_layer(
    State(cache): State<CacheState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Skip if L1 cache disabled
    if !cache.config.enable_l1_cache {
        return next.run(request).await;
    }

    // Only cache GET requests
    if request.method() != Method::GET {
        return next.run(request).await;
    }

    // Skip datastar streaming requests (SSE)
    if request.headers().contains_key("datastar-request") {
        return next.run(request).await;
    }

    // Build cache key
    let path = request.uri().path().to_string();
    let query = request.uri().query().unwrap_or("");
    let format = detect_format(&request);

    let l1_key = L1Key::Response {
        format,
        path: path.clone(),
        query_hash: hash_query(query),
    };

    // Check cache
    if let Some(cached) = cache.l1.get(&l1_key) {
        debug!(cache = "l1", outcome = "hit", "serving cached response");
        return build_response(cached);
    }

    debug!(
        cache = "l1",
        outcome = "miss",
        "cache miss, executing handler"
    );

    // Run with dependency collector
    let (response, deps) = deps::with_collector(next.run(request)).await;

    // Only cache successful responses (plus tag/month 404s)
    if should_cache_response(response.status(), &path) {
        let body_limit = cache.config.l1_response_body_limit_bytes;
        if body_limit == 0 {
            debug!(
                cache = "l1",
                outcome = "skip",
                reason = "body_limit_zero",
                "response body caching disabled by limit"
            );
            return response;
        }
        if let Some(content_length) = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok())
            && content_length > body_limit
        {
            debug!(
                cache = "l1",
                outcome = "skip",
                reason = "body_too_large",
                content_length,
                body_limit,
                "response body exceeds cache limit; skipping cache"
            );
            return response;
        }

        let (parts, body) = response.into_parts();
        let bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(b) => b,
            Err(_) => {
                // If body collection fails, return without caching
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

        if bytes.len() > body_limit {
            debug!(
                cache = "l1",
                outcome = "skip",
                reason = "body_too_large",
                body_len = bytes.len(),
                body_limit,
                "response body exceeds cache limit; skipping cache"
            );
            return Response::from_parts(parts, Body::from(bytes));
        }

        let cached = CachedResponse {
            status: parts.status.as_u16(),
            headers: parts
                .headers
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string())))
                .collect(),
            body: bytes.clone(),
        };

        debug!(cache = "l1", deps_count = deps.len(), "caching response");

        if let Some(evicted) = cache.l1.set(l1_key.clone(), cached) {
            cache.registry.unregister(&CacheKey::L1(evicted));
        }
        cache.registry.register(CacheKey::L1(l1_key), deps);

        Response::from_parts(parts, Body::from(bytes))
    } else {
        response
    }
}

/// Detect output format from request.
fn detect_format(request: &Request<Body>) -> OutputFormat {
    let path = request.uri().path();

    if path.ends_with("/rss.xml") || path == "/rss.xml" {
        OutputFormat::Rss
    } else if path.ends_with("/atom.xml") || path == "/atom.xml" {
        OutputFormat::Atom
    } else if path.ends_with("/sitemap.xml") || path == "/sitemap.xml" {
        OutputFormat::Sitemap
    } else if path.ends_with("/favicon.ico") || path == "/favicon.ico" {
        OutputFormat::Favicon
    } else if request
        .headers()
        .get("Accept")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/json"))
    {
        OutputFormat::Json
    } else {
        OutputFormat::Html
    }
}

fn should_cache_response(status: StatusCode, path: &str) -> bool {
    if status == StatusCode::OK {
        return true;
    }

    if status == StatusCode::NOT_FOUND {
        return is_tag_or_month_path(path);
    }

    false
}

fn is_tag_or_month_path(path: &str) -> bool {
    path.starts_with("/tags/") || path.starts_with("/months/")
}

/// Build a response from cached data.
fn build_response(cached: CachedResponse) -> Response {
    use axum::http::HeaderValue;

    let mut builder = Response::builder().status(cached.status);

    for (name, value) in cached.headers {
        if let Ok(header_value) = HeaderValue::from_str(&value) {
            builder = builder.header(name, header_value);
        }
    }

    builder
        .body(Body::from(cached.body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use axum::{
        Router,
        http::{Method, Request, StatusCode},
        middleware,
        routing::get,
    };
    use tower::ServiceExt;

    #[test]
    fn detect_format_html_default() {
        let req = Request::builder()
            .uri("/posts/hello")
            .body(Body::empty())
            .unwrap();
        assert_eq!(detect_format(&req), OutputFormat::Html);
    }

    #[test]
    fn detect_format_rss() {
        let req = Request::builder()
            .uri("/rss.xml")
            .body(Body::empty())
            .unwrap();
        assert_eq!(detect_format(&req), OutputFormat::Rss);
    }

    #[test]
    fn detect_format_atom() {
        let req = Request::builder()
            .uri("/atom.xml")
            .body(Body::empty())
            .unwrap();
        assert_eq!(detect_format(&req), OutputFormat::Atom);
    }

    #[test]
    fn detect_format_sitemap() {
        let req = Request::builder()
            .uri("/sitemap.xml")
            .body(Body::empty())
            .unwrap();
        assert_eq!(detect_format(&req), OutputFormat::Sitemap);
    }

    #[test]
    fn detect_format_json_from_accept_header() {
        let req = Request::builder()
            .uri("/posts/hello")
            .header("Accept", "application/json")
            .body(Body::empty())
            .unwrap();
        assert_eq!(detect_format(&req), OutputFormat::Json);
    }

    #[test]
    fn detect_format_favicon() {
        let req = Request::builder()
            .uri("/favicon.ico")
            .body(Body::empty())
            .unwrap();
        assert_eq!(detect_format(&req), OutputFormat::Favicon);
    }

    fn build_cache_state(
        mut config: CacheConfig,
    ) -> (CacheState, Arc<L1Store>, Arc<CacheRegistry>) {
        config.enable_l1_cache = true;
        let l1 = Arc::new(L1Store::new(&config));
        let registry = Arc::new(CacheRegistry::new());
        let state = CacheState {
            config,
            l1: l1.clone(),
            registry: registry.clone(),
        };
        (state, l1, registry)
    }

    #[tokio::test]
    async fn caches_tag_not_found_responses() {
        let config = CacheConfig {
            l1_response_body_limit_bytes: 1024,
            ..Default::default()
        };
        let (state, l1, _registry) = build_cache_state(config);

        let calls = Arc::new(AtomicUsize::new(0));
        let handler_calls = calls.clone();

        let app = Router::new()
            .route(
                "/tags/test",
                get(move || {
                    let handler_calls = handler_calls.clone();
                    async move {
                        handler_calls.fetch_add(1, Ordering::SeqCst);
                        crate::cache::deps::record(crate::cache::EntityKey::PostAggTags);
                        StatusCode::NOT_FOUND
                    }
                }),
            )
            .layer(middleware::from_fn_with_state(state, response_cache_layer));

        let request = Request::builder()
            .uri("/tags/test")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(l1.len(), 1);

        let request = Request::builder()
            .uri("/tags/test")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn skips_caching_when_body_exceeds_limit() {
        let config = CacheConfig {
            l1_response_body_limit_bytes: 4,
            ..Default::default()
        };
        let (state, l1, _registry) = build_cache_state(config);

        let calls = Arc::new(AtomicUsize::new(0));
        let handler_calls = calls.clone();

        let app = Router::new()
            .route(
                "/big",
                get(move || {
                    let handler_calls = handler_calls.clone();
                    async move {
                        handler_calls.fetch_add(1, Ordering::SeqCst);
                        Response::builder()
                            .status(StatusCode::OK)
                            .body(Body::from("12345"))
                            .unwrap()
                    }
                }),
            )
            .layer(middleware::from_fn_with_state(state, response_cache_layer));

        let request = Request::builder()
            .uri("/big")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(l1.len(), 0);

        let request = Request::builder()
            .uri("/big")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let _response = app.oneshot(request).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn evicted_entries_unregister_registry() {
        let config = CacheConfig {
            l1_response_limit: 1,
            l1_response_body_limit_bytes: 1024,
            ..Default::default()
        };
        let (state, l1, registry) = build_cache_state(config);

        let app = Router::new()
            .route(
                "/a",
                get(|| async {
                    crate::cache::deps::record(crate::cache::EntityKey::PostsIndex);
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("a"))
                        .unwrap()
                }),
            )
            .route(
                "/b",
                get(|| async {
                    crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("b"))
                        .unwrap()
                }),
            )
            .layer(middleware::from_fn_with_state(state, response_cache_layer));

        let request = Request::builder()
            .uri("/a")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let _response = app.clone().oneshot(request).await.unwrap();

        let request = Request::builder()
            .uri("/b")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let _response = app.oneshot(request).await.unwrap();

        assert_eq!(l1.len(), 1);
        assert_eq!(registry.key_count(), 1);
        assert!(
            registry
                .keys_for_entity(&crate::cache::EntityKey::PostsIndex)
                .is_empty()
        );
        assert_eq!(
            registry
                .keys_for_entity(&crate::cache::EntityKey::SiteSettings)
                .len(),
            1
        );
    }
}
