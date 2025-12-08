use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, StatusCode},
    response::Response,
};
use bytes::Bytes;
use http_body_util::BodyExt;
use thiserror::Error;
use tokio::sync::RwLock;

/// Default debounce window for cache warming.
/// Multiple write operations within this window will only trigger one warm.
pub const DEFAULT_CACHE_WARM_DEBOUNCE: Duration = Duration::from_secs(5);

/// Debouncer to prevent frequent cache warming operations.
///
/// When multiple write operations occur in quick succession, we only want to
/// warm the cache once after a brief delay, rather than warming after each operation.
#[derive(Clone, Default)]
pub struct CacheWarmDebouncer {
    last_warm: Arc<RwLock<Option<Instant>>>,
    debounce_window: Duration,
}

impl CacheWarmDebouncer {
    pub fn new(debounce_window: Duration) -> Self {
        Self {
            last_warm: Arc::new(RwLock::new(None)),
            debounce_window,
        }
    }

    /// Check if warming should proceed based on the debounce window.
    /// Returns true if enough time has passed since the last warm.
    pub async fn should_warm(&self) -> bool {
        let guard = self.last_warm.read().await;
        match *guard {
            Some(last) => last.elapsed() >= self.debounce_window,
            None => true,
        }
    }

    /// Mark that warming has started.
    /// Call this when a cache warm job is successfully enqueued.
    pub async fn mark_warm_requested(&self) {
        let mut guard = self.last_warm.write().await;
        *guard = Some(Instant::now());
    }

    /// Attempt to start warming if debounce window has passed.
    /// Returns true if warming should proceed, false if skipped due to debouncing.
    pub async fn try_warm(&self) -> bool {
        let mut guard = self.last_warm.write().await;
        let should_warm = match *guard {
            Some(last) => last.elapsed() >= self.debounce_window,
            None => true,
        };
        if should_warm {
            *guard = Some(Instant::now());
        }
        should_warm
    }
}

#[derive(Clone, Default)]
pub struct ResponseCache {
    entries: Arc<RwLock<HashMap<String, CachedResponse>>>,
    seo_entries: Arc<RwLock<HashMap<SeoKey, String>>>,
}

impl ResponseCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            seo_entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Response<Body>> {
        let guard = self.entries.read().await;
        guard.get(key).cloned().map(CachedResponse::into_response)
    }

    pub async fn put(&self, key: String, response: CachedResponse) {
        let mut guard = self.entries.write().await;
        guard.insert(key, response);
    }

    pub async fn store_response(
        &self,
        key: &str,
        response: Response,
    ) -> Result<Response, (Response, CacheStoreError)> {
        match buffer_response(response).await {
            Ok((rebuilt, cached)) => {
                self.put(key.to_string(), cached).await;
                Ok(rebuilt)
            }
            Err((rebuilt, error)) => Err((rebuilt, error)),
        }
    }

    pub async fn invalidate_all(&self) {
        let mut guard = self.entries.write().await;
        guard.clear();

        let mut seo_guard = self.seo_entries.write().await;
        seo_guard.clear();
    }

    pub async fn get_seo(&self, key: SeoKey) -> Option<String> {
        let guard = self.seo_entries.read().await;
        guard.get(&key).cloned()
    }

    pub async fn put_seo(&self, key: SeoKey, value: String) {
        let mut guard = self.seo_entries.write().await;
        guard.insert(key, value);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeoKey {
    Sitemap,
    Rss,
    Atom,
    Robots,
}

#[derive(Clone)]
pub struct CachedResponse {
    status: StatusCode,
    headers: Vec<(HeaderName, HeaderValue)>,
    body: Bytes,
}

impl CachedResponse {
    pub fn new(status: StatusCode, headers: &axum::http::HeaderMap, body: Bytes) -> Self {
        let mut stored_headers = Vec::with_capacity(headers.len());
        for (name, value) in headers.iter() {
            stored_headers.push((name.clone(), value.clone()));
        }

        Self {
            status,
            headers: stored_headers,
            body,
        }
    }

    fn into_response(self) -> Response<Body> {
        let mut response = Response::new(Body::from(self.body));
        *response.status_mut() = self.status;

        let headers = response.headers_mut();
        headers.clear();
        for (name, value) in self.headers {
            headers.append(name, value);
        }

        response
    }
}

#[derive(Debug, Error)]
pub enum CacheStoreError {
    #[error("failed to buffer response body: {0}")]
    Buffer(String),
}

pub fn should_store_response(response: &Response) -> bool {
    use axum::http::header;

    if !response.status().is_success() {
        return false;
    }

    if response.headers().contains_key(header::SET_COOKIE) {
        return false;
    }

    if response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/event-stream"))
    {
        return false;
    }

    true
}

pub async fn buffer_response(
    response: Response,
) -> Result<(Response, CachedResponse), (Response, CacheStoreError)> {
    let (parts, body) = response.into_parts();
    match BodyExt::collect(body).await {
        Ok(collected) => {
            let bytes = collected.to_bytes();
            let cached = CachedResponse::new(parts.status, &parts.headers, bytes.clone());
            let rebuilt = Response::from_parts(parts, Body::from(bytes));
            Ok((rebuilt, cached))
        }
        Err(error) => {
            let rebuilt = Response::from_parts(parts, Body::empty());
            Err((rebuilt, CacheStoreError::Buffer(error.to_string())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn debouncer_allows_first_warm() {
        let debouncer = CacheWarmDebouncer::new(Duration::from_secs(5));

        // First warm should always be allowed
        assert!(debouncer.should_warm().await);
        assert!(debouncer.try_warm().await);
    }

    #[tokio::test]
    async fn debouncer_blocks_rapid_requests() {
        let debouncer = CacheWarmDebouncer::new(Duration::from_millis(100));

        // First warm succeeds
        assert!(debouncer.try_warm().await);

        // Immediate second warm should be blocked
        assert!(!debouncer.should_warm().await);
        assert!(!debouncer.try_warm().await);
    }

    #[tokio::test]
    async fn debouncer_allows_after_window() {
        let debouncer = CacheWarmDebouncer::new(Duration::from_millis(50));

        // First warm succeeds
        assert!(debouncer.try_warm().await);

        // Wait for debounce window to pass
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should now be allowed
        assert!(debouncer.should_warm().await);
        assert!(debouncer.try_warm().await);
    }

    #[tokio::test]
    async fn debouncer_mark_warm_blocks_subsequent() {
        let debouncer = CacheWarmDebouncer::new(Duration::from_millis(100));

        // Mark that a warm was requested
        debouncer.mark_warm_requested().await;

        // Should be blocked
        assert!(!debouncer.should_warm().await);
    }
}
