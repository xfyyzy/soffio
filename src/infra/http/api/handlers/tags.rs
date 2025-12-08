//! Tags handlers

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::application::admin::tags::{CreateTagCommand, UpdateTagCommand};
use crate::application::api_keys::ApiPrincipal;
use crate::application::pagination::{PageRequest, TagCursor};
use crate::application::repos::TagQueryFilter;
use crate::domain::api_keys::ApiScope;

use super::{TagListQuery, settings_to_api, tag_to_api};
use crate::infra::http::api::error::ApiError;
use crate::infra::http::api::models::*;
use crate::infra::http::api::state::ApiState;

pub async fn list_tags(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Query(query): Query<TagListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::TagRead)
        .map_err(|_| ApiError::forbidden())?;
    let settings = state.settings.load().await.map_err(settings_to_api)?;
    let limit = query
        .limit
        .unwrap_or_else(|| settings.admin_page_size.max(1) as u32)
        .clamp(1, 100);

    let cursor = match query.cursor.as_deref().map(TagCursor::decode).transpose() {
        Ok(cursor) => cursor,
        Err(err) => {
            return Err(ApiError::bad_request(
                "invalid cursor",
                Some(err.to_string()),
            ));
        }
    };

    let filter = TagQueryFilter {
        search: query.search,
        month: query.month,
    };

    let page = state
        .tags
        .list(query.pinned, &filter, PageRequest::new(limit, cursor))
        .await
        .map_err(tag_to_api)?;

    Ok(Json(page))
}

pub async fn get_tag_by_id(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::TagRead)
        .map_err(|_| ApiError::forbidden())?;

    let tag = state.tags.find_by_id(id).await.map_err(tag_to_api)?;

    match tag {
        Some(tag) => Ok(Json(tag)),
        None => Err(ApiError::not_found("tag not found")),
    }
}

pub async fn get_tag_by_slug(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::TagRead)
        .map_err(|_| ApiError::forbidden())?;

    let tag = state.tags.find_by_slug(&slug).await.map_err(tag_to_api)?;

    match tag {
        Some(tag) => Ok(Json(tag)),
        None => Err(ApiError::not_found("tag not found")),
    }
}

pub async fn create_tag(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Json(payload): Json<TagCreateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::TagWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let command = CreateTagCommand {
        name: payload.name,
        description: payload.description,
        pinned: payload.pinned,
    };

    let tag = state
        .tags
        .create_tag(&actor, command)
        .await
        .map_err(tag_to_api)?;

    Ok((StatusCode::CREATED, Json(tag)))
}

pub async fn update_tag(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<TagUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::TagWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let command = UpdateTagCommand {
        id,
        name: payload.name,
        description: payload.description,
        pinned: payload.pinned,
    };

    let tag = state
        .tags
        .update_tag(&actor, command)
        .await
        .map_err(tag_to_api)?;

    Ok(Json(tag))
}

pub async fn update_tag_pin(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<TagPinRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::TagWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let tag = state
        .tags
        .update_tag_pinned(&actor, id, payload.pinned)
        .await
        .map_err(tag_to_api)?;

    Ok(Json(tag))
}

pub async fn update_tag_name(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<TagNameRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::TagWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let existing = state
        .tags
        .find_by_id(id)
        .await
        .map_err(tag_to_api)?
        .ok_or_else(|| ApiError::not_found("tag not found"))?;

    let command = UpdateTagCommand {
        id,
        name: payload.name,
        description: existing.description.clone(),
        pinned: existing.pinned,
    };

    let tag = state
        .tags
        .update_tag(&actor, command)
        .await
        .map_err(tag_to_api)?;

    Ok(Json(tag))
}

pub async fn update_tag_description(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<TagDescriptionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::TagWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let existing = state
        .tags
        .find_by_id(id)
        .await
        .map_err(tag_to_api)?
        .ok_or_else(|| ApiError::not_found("tag not found"))?;

    let command = UpdateTagCommand {
        id,
        name: existing.name.clone(),
        description: payload.description,
        pinned: existing.pinned,
    };

    let tag = state
        .tags
        .update_tag(&actor, command)
        .await
        .map_err(tag_to_api)?;

    Ok(Json(tag))
}

pub async fn delete_tag(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::TagWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    state
        .tags
        .delete_tag(&actor, id)
        .await
        .map_err(tag_to_api)?;

    Ok(StatusCode::NO_CONTENT)
}
