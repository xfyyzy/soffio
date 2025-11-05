use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, StatusCode},
    response::Response,
};
use bytes::Bytes;
use http_body_util::BodyExt;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct ResponseCache {
    entries: Arc<RwLock<HashMap<String, CachedResponse>>>,
}

impl ResponseCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
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
    }
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
