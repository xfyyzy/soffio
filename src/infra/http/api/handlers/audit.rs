//! Audit handlers

use axum::Json;
use axum::extract::{Extension, Query, State};
use axum::response::IntoResponse;

use crate::application::api_keys::ApiPrincipal;
use crate::application::pagination::{AuditCursor, PageRequest};
use crate::application::repos::AuditQueryFilter;
use crate::domain::api_keys::ApiScope;

use super::{AuditListQuery, repo_to_api};
use crate::infra::http::api::error::ApiError;
use crate::infra::http::api::state::ApiState;

pub async fn list_audit_logs(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Query(query): Query<AuditListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::AuditRead)
        .map_err(|_| ApiError::forbidden())?;

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let cursor = match query.cursor.as_deref().map(AuditCursor::decode).transpose() {
        Ok(cursor) => cursor,
        Err(err) => {
            return Err(ApiError::bad_request(
                "invalid cursor",
                Some(err.to_string()),
            ));
        }
    };

    let filter = AuditQueryFilter {
        actor: query.actor,
        action: query.action,
        entity_type: query.entity_type,
        search: query.search,
    };

    let page = state
        .audit
        .list_filtered(PageRequest::new(limit, cursor), &filter)
        .await
        .map_err(repo_to_api)?;

    Ok(Json(page))
}
