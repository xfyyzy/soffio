use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tracing::warn;

use crate::application::api_keys::ApiAuthError;

use super::error::ApiError;
use super::state::ApiState;

pub async fn api_auth(
    State(state): State<ApiState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let token =
        extract_token(request.headers().get(axum::http::header::AUTHORIZATION)).or_else(|| {
            request
                .headers()
                .get("x-api-key")
                .and_then(|v| v.to_str().ok().map(|s| s.to_string()))
        });

    let token = match token {
        Some(value) => value,
        None => return ApiError::unauthorized().into_response(),
    };

    let principal = match state.api_keys.authenticate(&token).await {
        Ok(principal) => principal,
        Err(ApiAuthError::Missing) | Err(ApiAuthError::Invalid) => {
            return ApiError::unauthorized().into_response();
        }
        Err(ApiAuthError::Expired) => {
            return ApiError::new(StatusCode::UNAUTHORIZED, "expired", "API key expired", None)
                .into_response();
        }
        Err(ApiAuthError::Revoked) => {
            return ApiError::new(StatusCode::UNAUTHORIZED, "revoked", "API key revoked", None)
                .into_response();
        }
    };

    request.extensions_mut().insert(principal);
    request.extensions_mut().insert(state.clone());

    next.run(request).await
}

pub async fn api_rate_limit(
    State(state): State<ApiState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let principal = match request
        .extensions()
        .get::<crate::application::api_keys::ApiPrincipal>()
    {
        Some(p) => p,
        None => {
            warn!(
                target = "soffio::api::ratelimit",
                "missing principal in rate limit middleware"
            );
            return ApiError::unauthorized().into_response();
        }
    };

    let key = principal.key_id.to_string();

    if !state.rate_limiter.allow(&key, &path) {
        return ApiError::rate_limited(state.rate_limiter.retry_after_secs());
    }

    next.run(request).await
}

fn extract_token(header: Option<&axum::http::HeaderValue>) -> Option<String> {
    let raw = header?.to_str().ok()?;
    let bearer = raw.strip_prefix("Bearer ")?;
    Some(bearer.to_string())
}
