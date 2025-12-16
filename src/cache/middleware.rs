//! L1 response cache middleware.
//!
//! Caches GET requests to public routes and serves cached responses.
//! Skips caching for datastar streaming requests (SSE).

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
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
/// Only caches GET requests to public routes that return 200 OK.
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

    // Only cache successful responses
    if response.status() == StatusCode::OK {
        let (parts, body) = response.into_parts();
        let bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
            Ok(b) => b,
            Err(_) => {
                // If body collection fails, return without caching
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

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

        cache.l1.set(l1_key.clone(), cached);
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
    use axum::http::Request;

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
}
