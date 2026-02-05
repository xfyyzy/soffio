//! Pages handlers

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::application::admin::pages::{
    CreatePageCommand, UpdatePageContentCommand, UpdatePageStatusCommand,
};
use crate::application::api_keys::ApiPrincipal;
use crate::application::pagination::PageCursor;
use crate::application::repos::PageQueryFilter;
use crate::domain::api_keys::ApiScope;

use super::{PageListQuery, page_to_api, settings_to_api};
use crate::infra::http::api::error::ApiError;
use crate::infra::http::api::models::*;
use crate::infra::http::api::state::ApiState;

pub async fn list_pages(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Query(query): Query<PageListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PageRead)
        .map_err(|_| ApiError::forbidden())?;

    let settings = state.settings.load().await.map_err(settings_to_api)?;
    let limit = query
        .limit
        .unwrap_or_else(|| settings.admin_page_size.max(1) as u32)
        .clamp(1, 100);

    let cursor = match query.cursor.as_deref().map(PageCursor::decode).transpose() {
        Ok(cursor) => cursor,
        Err(err) => {
            return Err(ApiError::bad_request(
                "invalid cursor",
                Some(err.to_string()),
            ));
        }
    };

    let filter = PageQueryFilter {
        search: query.search,
        month: query.month,
    };

    let page = state
        .pages
        .list(query.status, limit, cursor, &filter)
        .await
        .map_err(page_to_api)?;

    Ok(Json(page))
}

pub async fn get_page(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PageRead)
        .map_err(|_| ApiError::forbidden())?;

    let page = state.pages.find_by_slug(&slug).await.map_err(page_to_api)?;

    match page {
        Some(page) => Ok(Json(page)),
        None => Err(ApiError::not_found("page not found")),
    }
}

pub async fn get_page_by_id(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PageRead)
        .map_err(|_| ApiError::forbidden())?;

    let page = state.pages.find_by_id(id).await.map_err(page_to_api)?;

    match page {
        Some(page) => Ok(Json(page)),
        None => Err(ApiError::not_found("page not found")),
    }
}

pub async fn create_page(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Json(payload): Json<PageCreateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PageWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let command = CreatePageCommand {
        slug: payload.slug,
        title: payload.title,
        body_markdown: payload.body_markdown,
        status: payload.status,
        scheduled_at: payload.scheduled_at,
        published_at: payload.published_at,
        archived_at: payload.archived_at,
    };

    let page = state
        .pages
        .create_page(&actor, command)
        .await
        .map_err(page_to_api)?;

    Ok((StatusCode::CREATED, Json(page)))
}

pub async fn update_page(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PageUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PageWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let command = UpdatePageContentCommand {
        id,
        slug: payload.slug,
        title: payload.title,
        body_markdown: payload.body_markdown,
    };

    let page = state
        .pages
        .update_page(&actor, command)
        .await
        .map_err(page_to_api)?;

    Ok(Json(page))
}

pub async fn update_page_title(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PageTitleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PageWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let page = state
        .pages
        .find_by_id(id)
        .await
        .map_err(page_to_api)?
        .ok_or_else(|| ApiError::not_found("page not found"))?;

    if payload.title.trim().is_empty() {
        return Err(ApiError::bad_request("title cannot be empty", None));
    }

    let command = UpdatePageContentCommand {
        id,
        slug: page.slug.clone(),
        title: payload.title,
        body_markdown: page.body_markdown.clone(),
    };

    let updated = state
        .pages
        .update_page(&actor, command)
        .await
        .map_err(page_to_api)?;

    Ok(Json(updated))
}

pub async fn update_page_body(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PageBodyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PageWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let page = state
        .pages
        .find_by_id(id)
        .await
        .map_err(page_to_api)?
        .ok_or_else(|| ApiError::not_found("page not found"))?;

    let command = UpdatePageContentCommand {
        id,
        slug: page.slug.clone(),
        title: page.title.clone(),
        body_markdown: payload.body_markdown,
    };

    let updated = state
        .pages
        .update_page(&actor, command)
        .await
        .map_err(page_to_api)?;

    Ok(Json(updated))
}

pub async fn update_page_status(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PageStatusRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PageWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let command = UpdatePageStatusCommand {
        id,
        status: payload.status,
        scheduled_at: payload.scheduled_at,
        published_at: payload.published_at,
        archived_at: payload.archived_at,
    };

    let page = state
        .pages
        .update_status(&actor, command)
        .await
        .map_err(page_to_api)?;

    Ok(Json(page))
}

pub async fn delete_page(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PageWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let page = state
        .pages
        .find_by_id(id)
        .await
        .map_err(page_to_api)?
        .ok_or_else(|| ApiError::not_found("page not found"))?;

    state
        .pages
        .delete_page(&actor, id, &page.slug)
        .await
        .map_err(page_to_api)?;

    Ok(StatusCode::NO_CONTENT)
}
