use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::application::admin::snapshot_types::{
    PageSnapshotPayload, PageSnapshotSource, PostSnapshotPayload, PostSnapshotSource,
};
use crate::application::admin::snapshots::SnapshotServiceError;
use crate::application::api_keys::ApiPrincipal;
use crate::application::pagination::{CursorPage, PageRequest, SnapshotCursor};
use crate::application::repos::{SnapshotFilter, SnapshotRecord};
use crate::domain::types::SnapshotEntityType;

use super::super::error::ApiError;
use super::super::models::{SnapshotCreateRequest, SnapshotListQuery, SnapshotResponse};
use super::{page_to_api, post_to_api, snapshot_to_api};
use crate::application::error::AppError;
use crate::infra::http::api::state::ApiState;

fn record_to_response(record: SnapshotRecord) -> SnapshotResponse {
    SnapshotResponse {
        id: record.id,
        entity_type: record.entity_type,
        entity_id: record.entity_id,
        version: record.version,
        description: record.description,
        schema_version: record.schema_version,
        content: record.content,
        created_by: record.created_by,
        created_at: record.created_at,
    }
}

pub async fn list_snapshots(
    State(state): State<ApiState>,
    Query(query): Query<SnapshotListQuery>,
    axum::extract::Extension(principal): axum::extract::Extension<ApiPrincipal>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::SnapshotRead)
        .map_err(|_| ApiError::forbidden())?;

    let cursor = match query
        .cursor
        .as_deref()
        .map(SnapshotCursor::decode)
        .transpose()
    {
        Ok(c) => c,
        Err(err) => {
            return Err(ApiError::bad_request(
                "invalid cursor",
                Some(err.to_string()),
            ));
        }
    };

    let filter = SnapshotFilter {
        entity_type: query.entity_type,
        entity_id: query.entity_id,
        search: query.search,
        month: None,
    };

    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let page = state
        .snapshots
        .list(&filter, PageRequest::new(limit, cursor))
        .await
        .map_err(snapshot_to_api)?;

    let response = CursorPage {
        items: page.items.into_iter().map(record_to_response).collect(),
        next_cursor: page.next_cursor,
    };

    Ok(Json(response))
}

pub async fn get_snapshot(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    axum::extract::Extension(principal): axum::extract::Extension<ApiPrincipal>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::SnapshotRead)
        .map_err(|_| ApiError::forbidden())?;

    let snapshot = state
        .snapshots
        .find(id)
        .await
        .map_err(snapshot_to_api)?
        .ok_or_else(|| ApiError::not_found("snapshot not found"))?;

    Ok(Json(record_to_response(snapshot)))
}

pub async fn create_snapshot(
    State(state): State<ApiState>,
    axum::extract::Extension(principal): axum::extract::Extension<ApiPrincipal>,
    Json(payload): Json<SnapshotCreateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::SnapshotWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let record = match payload.entity_type {
        SnapshotEntityType::Post => {
            let source = state
                .posts
                .snapshot_source(payload.entity_id)
                .await
                .map_err(post_to_api)?;
            state
                .snapshots
                .create::<PostSnapshotSource>(&actor, &source, payload.description)
                .await
        }
        SnapshotEntityType::Page => {
            let source = state
                .pages
                .snapshot_source(payload.entity_id)
                .await
                .map_err(page_to_api)?;
            state
                .snapshots
                .create::<PageSnapshotSource>(&actor, &source, payload.description)
                .await
        }
    }
    .map_err(snapshot_to_api)?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(record_to_response(record)),
    ))
}

pub async fn rollback_snapshot(
    State(state): State<ApiState>,
    axum::extract::Extension(principal): axum::extract::Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::SnapshotWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    // Decide entity type by reading snapshot first
    let snapshot = state
        .snapshots
        .find(id)
        .await
        .map_err(snapshot_to_api)?
        .ok_or_else(|| ApiError::not_found("snapshot not found"))?;

    let record = match snapshot.entity_type {
        SnapshotEntityType::Post => {
            state
                .snapshots
                .rollback::<PostSnapshotSource, _, _>(&actor, id, |payload| async {
                    let post_payload: PostSnapshotPayload = payload;
                    state
                        .posts
                        .restore_from_snapshot(post_payload, snapshot.entity_id)
                        .await
                        .map(|_| ())
                        .map_err(|e| match e {
                            crate::application::admin::posts::types::AdminPostError::ConstraintViolation(field) => {
                                SnapshotServiceError::Snapshot(
                                    crate::domain::snapshots::SnapshotError::Validation(field.to_string()),
                                )
                            }
                            crate::application::admin::posts::types::AdminPostError::Repo(repo) => {
                                SnapshotServiceError::Repo(repo)
                            }
                        })
                })
                .await
        }
        SnapshotEntityType::Page => {
            state
                .snapshots
                .rollback::<PageSnapshotSource, _, _>(&actor, id, |payload| async {
                    let page_payload: PageSnapshotPayload = payload;
                    state
                        .pages
                        .restore_from_snapshot(page_payload, snapshot.entity_id)
                        .await
                        .map(|_| ())
                        .map_err(|e| match e {
                            crate::application::admin::pages::AdminPageError::ConstraintViolation(field) => {
                                SnapshotServiceError::Snapshot(
                                    crate::domain::snapshots::SnapshotError::Validation(field.to_string()),
                                )
                            }
                            crate::application::admin::pages::AdminPageError::Render(render_err) => {
                                SnapshotServiceError::App(AppError::unexpected(render_err.to_string()))
                            }
                            crate::application::admin::pages::AdminPageError::Repo(repo) => {
                                SnapshotServiceError::Repo(repo)
                            }
                        })
                })
                .await
        }
    }
    .map_err(snapshot_to_api)?;

    Ok(Json(record_to_response(record)))
}
