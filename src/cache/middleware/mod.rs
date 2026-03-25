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
use metrics::counter;
use tracing::{debug, instrument};

use super::{
    CacheConfig, CacheRegistry, L1Store, deps,
    keys::{CacheKey, L1Key, OutputFormat, hash_query},
    store::CachedResponse,
};

#[cfg(test)]
mod tests;

const METRIC_L1_HIT_TOTAL: &str = "soffio_cache_l1_hit_total";
const METRIC_L1_MISS_TOTAL: &str = "soffio_cache_l1_miss_total";

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
    let format_label = output_format_label(&format);

    let l1_key = L1Key::Response {
        format,
        path: path.clone(),
        query_hash: hash_query(query),
    };

    // Check cache
    if let Some(cached) = cache.l1.get(&l1_key) {
        counter!(METRIC_L1_HIT_TOTAL, "format" => format_label).increment(1);
        debug!(cache = "l1", outcome = "hit", "serving cached response");
        return build_response(cached);
    }

    counter!(METRIC_L1_MISS_TOTAL, "format" => format_label).increment(1);
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

fn output_format_label(format: &OutputFormat) -> &'static str {
    match format {
        OutputFormat::Html => "html",
        OutputFormat::Json => "json",
        OutputFormat::Rss => "rss",
        OutputFormat::Atom => "atom",
        OutputFormat::Sitemap => "sitemap",
        OutputFormat::Favicon => "favicon",
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
