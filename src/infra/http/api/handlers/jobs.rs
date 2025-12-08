//! Jobs handlers

use axum::Json;
use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::application::api_keys::ApiPrincipal;
use crate::application::pagination::{JobCursor, PageRequest};
use crate::application::repos::JobQueryFilter;
use crate::domain::api_keys::ApiScope;

use super::JobsListQuery;
use crate::infra::http::api::error::{ApiError, codes};
use crate::infra::http::api::state::ApiState;

pub async fn list_jobs(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Query(query): Query<JobsListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::JobRead)
        .map_err(|_| ApiError::forbidden())?;

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let cursor = match query.cursor.as_deref().map(JobCursor::decode).transpose() {
        Ok(cursor) => cursor,
        Err(err) => {
            return Err(ApiError::bad_request(
                "invalid cursor",
                Some(err.to_string()),
            ));
        }
    };

    let filter = JobQueryFilter {
        state: query.state,
        job_type: query.job_type,
        search: query.search,
    };

    let page = state
        .jobs
        .list_jobs(&filter, PageRequest::new(limit, cursor))
        .await
        .map_err(|err| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                codes::JOBS,
                "Failed to list jobs",
                Some(err.to_string()),
            )
        })?;

    Ok(Json(page))
}
