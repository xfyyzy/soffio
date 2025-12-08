//! API Key handlers

use axum::Json;
use axum::extract::{Extension, State};

use crate::application::api_keys::ApiPrincipal;

use super::api_key_to_api;
use crate::infra::http::api::error::ApiError;
use crate::infra::http::api::models::ApiKeyInfoResponse;
use crate::infra::http::api::state::ApiState;

pub async fn get_api_key_info(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
) -> Result<Json<ApiKeyInfoResponse>, ApiError> {
    let record = state
        .api_keys
        .load(principal.key_id)
        .await
        .map_err(api_key_to_api)?
        .ok_or_else(|| ApiError::not_found("api key not found"))?;

    let body = ApiKeyInfoResponse {
        name: record.name,
        prefix: record.prefix,
        scopes: record.scopes,
        status: record.status,
        expires_at: record.expires_at,
        revoked_at: record.revoked_at,
        last_used_at: record.last_used_at,
    };

    Ok(Json(body))
}
