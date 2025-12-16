//! Posts handlers

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::application::admin::posts::{
    CreatePostCommand, UpdatePostContentCommand, UpdatePostStatusCommand,
};
use crate::application::api_keys::ApiPrincipal;
use crate::application::pagination::{PageRequest, PostCursor};
use crate::application::repos::{PostListScope, PostQueryFilter};
use crate::domain::api_keys::ApiScope;

use super::{PostListQuery, post_to_api, repo_to_api, settings_to_api};
use crate::infra::http::api::error::ApiError;
use crate::infra::http::api::models::*;
use crate::infra::http::api::state::ApiState;

pub async fn list_posts(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Query(query): Query<PostListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostRead)
        .map_err(|_| ApiError::forbidden())?;

    let settings = state.settings.load().await.map_err(settings_to_api)?;
    let limit = query
        .limit
        .unwrap_or_else(|| settings.admin_page_size.max(1) as u32)
        .clamp(1, 100);

    let cursor = match query.cursor.as_deref().map(PostCursor::decode).transpose() {
        Ok(cursor) => cursor,
        Err(err) => {
            return Err(ApiError::bad_request(
                "invalid cursor",
                Some(err.to_string()),
            ));
        }
    };

    let filter = PostQueryFilter {
        tag: query.tag,
        month: query.month,
        search: query.search,
    };

    let page = state
        .posts
        .reader
        .list_posts(
            PostListScope::Admin {
                status: query.status,
            },
            &filter,
            PageRequest::new(limit, cursor),
        )
        .await
        .map_err(repo_to_api)?;

    Ok(Json(page))
}

pub async fn get_post(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostRead)
        .map_err(|_| ApiError::forbidden())?;

    let post = state
        .posts
        .reader
        .find_by_slug(&slug)
        .await
        .map_err(repo_to_api)?;

    match post {
        Some(post) => Ok(Json(post)),
        None => Err(ApiError::not_found("post not found")),
    }
}

pub async fn get_post_by_id(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostRead)
        .map_err(|_| ApiError::forbidden())?;

    let post = state
        .posts
        .reader
        .find_by_id(id)
        .await
        .map_err(repo_to_api)?;

    match post {
        Some(post) => Ok(Json(post)),
        None => Err(ApiError::not_found("post not found")),
    }
}

pub async fn create_post(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Json(payload): Json<PostCreateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;

    let actor = ApiState::actor_label(&principal);

    let command = CreatePostCommand {
        title: payload.title,
        excerpt: payload.excerpt,
        body_markdown: payload.body_markdown,
        summary_markdown: payload.summary_markdown,
        status: payload.status,
        pinned: payload.pinned,
        scheduled_at: payload.scheduled_at,
        published_at: payload.published_at,
        archived_at: payload.archived_at,
    };

    let post = state
        .posts
        .create_post(&actor, command)
        .await
        .map_err(post_to_api)?;

    Ok((StatusCode::CREATED, Json(post)))
}

pub async fn update_post(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PostUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let command = UpdatePostContentCommand {
        id,
        slug: payload.slug,
        title: payload.title,
        excerpt: payload.excerpt,
        body_markdown: payload.body_markdown,
        pinned: payload.pinned,
        summary_markdown: payload.summary_markdown,
    };

    let post = state
        .posts
        .update_post(&actor, command)
        .await
        .map_err(post_to_api)?;

    Ok(Json(post))
}

pub async fn update_post_status(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PostStatusRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let command = UpdatePostStatusCommand {
        id,
        status: payload.status,
        scheduled_at: payload.scheduled_at,
        published_at: payload.published_at,
        archived_at: payload.archived_at,
    };

    let post = state
        .posts
        .update_status(&actor, command)
        .await
        .map_err(post_to_api)?;

    Ok(Json(post))
}

pub async fn update_post_pin(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PostPinRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let post = state
        .posts
        .update_pin_state(&actor, id, payload.pinned)
        .await
        .map_err(post_to_api)?;

    Ok(Json(post))
}

pub async fn update_post_title(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PostTitleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let post = state
        .posts
        .load_post(id)
        .await
        .map_err(post_to_api)?
        .ok_or_else(|| ApiError::not_found("post not found"))?;

    if payload.title.trim().is_empty() {
        return Err(ApiError::bad_request("title cannot be empty", None));
    }

    let command = UpdatePostContentCommand {
        id,
        slug: post.slug.clone(),
        title: payload.title,
        excerpt: post.excerpt.clone(),
        body_markdown: post.body_markdown.clone(),
        pinned: post.pinned,
        summary_markdown: post.summary_markdown.clone(),
    };

    let updated = state
        .posts
        .update_post(&actor, command)
        .await
        .map_err(post_to_api)?;

    Ok(Json(updated))
}

pub async fn update_post_excerpt(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PostExcerptRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let post = state
        .posts
        .load_post(id)
        .await
        .map_err(post_to_api)?
        .ok_or_else(|| ApiError::not_found("post not found"))?;

    let command = UpdatePostContentCommand {
        id,
        slug: post.slug.clone(),
        title: post.title.clone(),
        excerpt: payload.excerpt,
        body_markdown: post.body_markdown.clone(),
        pinned: post.pinned,
        summary_markdown: post.summary_markdown.clone(),
    };

    let updated = state
        .posts
        .update_post(&actor, command)
        .await
        .map_err(post_to_api)?;

    Ok(Json(updated))
}

pub async fn update_post_body(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PostBodyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let post = state
        .posts
        .load_post(id)
        .await
        .map_err(post_to_api)?
        .ok_or_else(|| ApiError::not_found("post not found"))?;

    let command = UpdatePostContentCommand {
        id,
        slug: post.slug.clone(),
        title: post.title.clone(),
        excerpt: post.excerpt.clone(),
        body_markdown: payload.body_markdown,
        pinned: post.pinned,
        summary_markdown: post.summary_markdown.clone(),
    };

    let updated = state
        .posts
        .update_post(&actor, command)
        .await
        .map_err(post_to_api)?;

    Ok(Json(updated))
}

pub async fn update_post_summary(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PostSummaryRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let post = state
        .posts
        .load_post(id)
        .await
        .map_err(post_to_api)?
        .ok_or_else(|| ApiError::not_found("post not found"))?;

    let command = UpdatePostContentCommand {
        id,
        slug: post.slug.clone(),
        title: post.title.clone(),
        excerpt: post.excerpt.clone(),
        body_markdown: post.body_markdown.clone(),
        pinned: post.pinned,
        summary_markdown: payload.summary_markdown,
    };

    let updated = state
        .posts
        .update_post(&actor, command)
        .await
        .map_err(post_to_api)?;

    Ok(Json(updated))
}

pub async fn replace_post_tags(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PostTagsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let post = state
        .posts
        .reader
        .find_by_id(id)
        .await
        .map_err(repo_to_api)?
        .ok_or_else(|| ApiError::not_found("post not found"))?;

    state
        .posts
        .replace_tags(&actor, &post, &payload.tag_ids)
        .await
        .map_err(post_to_api)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_post(
    State(state): State<ApiState>,
    Extension(principal): Extension<ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = ApiState::actor_label(&principal);

    let post = state
        .posts
        .reader
        .find_by_id(id)
        .await
        .map_err(repo_to_api)?
        .ok_or_else(|| ApiError::not_found("post not found"))?;

    state
        .posts
        .delete_post(&actor, id, &post.slug)
        .await
        .map_err(post_to_api)?;

    Ok(StatusCode::NO_CONTENT)
}
