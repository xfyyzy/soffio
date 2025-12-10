use std::{sync::Arc, time::Instant};

use axum::{
    body::Body,
    extract::State,
    http::{Method, Request},
    middleware::Next,
    response::Response,
};
use tracing::{error, warn};
use uuid::Uuid;

use crate::{
    application::api_keys::ApiPrincipal,
    application::error::ErrorReport,
    infra::cache::{CacheWarmDebouncer, ResponseCache, should_store_response},
    infra::db::PostgresRepositories,
};

use super::DATASTAR_REQUEST_HEADER;

#[derive(Clone)]
pub struct RequestContext {
    pub request_id: String,
}

pub async fn set_request_context(mut request: Request<Body>, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let ctx = RequestContext {
        request_id: request_id.clone(),
    };
    request.extensions_mut().insert(ctx.clone());

    let mut response = next.run(request).await;
    response.extensions_mut().insert(ctx);
    response
}

pub async fn cache_public_responses(
    State(cache): State<Arc<ResponseCache>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if should_bypass_cache(&request) {
        return next.run(request).await;
    }

    let key = cache_key(&request);
    if let Some(response) = cache.get(&key).await {
        return response;
    }

    tracing::info!(
        target = "soffio::http::cache",
        path = %key,
        "cache miss"
    );

    let response = next.run(request).await;

    if !should_store_response(&response) {
        return response;
    }

    match cache.store_response(&key, response).await {
        Ok(rebuilt) => rebuilt,
        Err((rebuilt, error)) => {
            warn!(
                target = "soffio::http::cache",
                path = %key,
                error = %error,
                "failed to store cached response"
            );
            rebuilt
        }
    }
}

pub async fn invalidate_admin_writes(
    State(cache): State<Arc<ResponseCache>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let response = next.run(request).await;

    if method != Method::GET && response.status().is_success() {
        cache.invalidate_all().await;
    }

    response
}

/// State for cache invalidation with async warming support.
#[derive(Clone)]
pub struct CacheInvalidationState {
    pub cache: Arc<ResponseCache>,
    pub debouncer: Arc<CacheWarmDebouncer>,
    pub jobs_repo: Arc<PostgresRepositories>,
}

/// Middleware that invalidates cache on successful writes and triggers async warming.
///
/// This is used by API routes to ensure cache consistency after modifications
/// via soffio-cli or other API consumers.
pub async fn invalidate_and_warm_cache(
    State(state): State<CacheInvalidationState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    use crate::application::jobs::invalidate_and_enqueue_warm;
    use crate::application::repos::JobsRepo;

    let method = request.method().clone();
    let response = next.run(request).await;

    if method != Method::GET && response.status().is_success() {
        let cache = state.cache.clone();
        let debouncer = state.debouncer.clone();
        let jobs_repo = state.jobs_repo.clone();

        tokio::spawn(async move {
            if let Err(err) = invalidate_and_enqueue_warm(
                cache.as_ref(),
                &debouncer,
                jobs_repo.as_ref() as &dyn JobsRepo,
                Some("api_write".to_string()),
            )
            .await
            {
                tracing::warn!(
                    target = "infra::http::middleware",
                    error = %err,
                    "failed to invalidate and enqueue cache warm job"
                );
            }
        });
    }

    response
}

pub async fn log_responses(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    let (api_key_id, api_scopes) = match request.extensions().get::<ApiPrincipal>() {
        Some(principal) => (
            Some(principal.key_id.to_string()),
            Some(
                principal
                    .scopes
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ),
        None => (None, None),
    };

    let request_id = request
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.request_id.clone())
        .unwrap_or_default();

    let mut response = next.run(request).await;
    let status = response.status();

    if status.is_client_error() || status.is_server_error() {
        let elapsed_ms = start.elapsed().as_millis();
        let report = response.extensions_mut().remove::<ErrorReport>();
        let (source, messages) = match report {
            Some(report) => (report.source, report.messages),
            None => ("unknown", Vec::new()),
        };
        let detail = messages
            .first()
            .cloned()
            .unwrap_or_else(|| "no diagnostic available".to_string());

        if status.is_server_error() {
            error!(
                target = "soffio::http::response",
                status = status.as_u16(),
                method = %method,
                path = %uri.path(),
                query = uri.query().unwrap_or(""),
                elapsed_ms = elapsed_ms,
                source = source,
                detail = %detail,
                chain = ?messages,
                request_id = request_id,
                api_key_id = api_key_id.as_deref().unwrap_or(""),
                api_scopes = api_scopes.as_deref().unwrap_or(""),
                "request failed",
            );
        } else {
            warn!(
                target = "soffio::http::response",
                status = status.as_u16(),
                method = %method,
                path = %uri.path(),
                query = uri.query().unwrap_or(""),
                elapsed_ms = elapsed_ms,
                source = source,
                detail = %detail,
                chain = ?messages,
                request_id = request_id,
                api_key_id = api_key_id.as_deref().unwrap_or(""),
                api_scopes = api_scopes.as_deref().unwrap_or(""),
                "client request error",
            );
        }
    }

    response
}

fn should_bypass_cache(request: &Request<Body>) -> bool {
    if request.method() != Method::GET {
        return true;
    }

    request.headers().contains_key(DATASTAR_REQUEST_HEADER)
}

fn cache_key(request: &Request<Body>) -> String {
    request
        .uri()
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string())
}
