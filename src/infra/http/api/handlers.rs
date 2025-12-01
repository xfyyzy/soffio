use axum::Json;
use axum::extract::{Extension, Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use uuid::Uuid;

use crate::application::admin::navigation::{
    AdminNavigationError, CreateNavigationItemCommand, UpdateNavigationItemCommand,
};
use crate::application::admin::pages::{
    AdminPageError, CreatePageCommand, UpdatePageContentCommand, UpdatePageStatusCommand,
};
use crate::application::admin::posts::{
    AdminPostError, CreatePostCommand, UpdatePostContentCommand, UpdatePostStatusCommand,
};
use crate::application::admin::settings::AdminSettingsError;
use crate::application::admin::tags::{AdminTagError, CreateTagCommand, UpdateTagCommand};
use crate::application::admin::uploads::AdminUploadError;
use crate::application::pagination::{
    JobCursor, NavigationCursor, PageCursor, PageRequest, PostCursor, TagCursor, UploadCursor,
};
use crate::application::repos::{
    AuditQueryFilter, JobQueryFilter, NavigationQueryFilter, PageQueryFilter, PostListScope,
    PostQueryFilter, RepoError, TagQueryFilter, UploadQueryFilter,
};
use crate::domain::entities::UploadRecord;
use crate::domain::types::{JobState, JobType, PageStatus, PostStatus};
use crate::infra::uploads::UploadStorageError;
use time::OffsetDateTime;

use super::error::{ApiError, codes};
use super::models::*;
use super::state::ApiState;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CursorQuery {
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PostListQuery {
    pub status: Option<PostStatus>,
    pub search: Option<String>,
    pub tag: Option<String>,
    pub month: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct PageListQuery {
    pub status: Option<PageStatus>,
    pub search: Option<String>,
    pub month: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct TagListQuery {
    pub search: Option<String>,
    pub month: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub pinned: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct NavigationListQuery {
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub visible: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UploadListQuery {
    pub search: Option<String>,
    pub content_type: Option<String>,
    pub month: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct JobsListQuery {
    pub state: Option<JobState>,
    pub job_type: Option<JobType>,
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct AuditListQuery {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

/// -------- Posts --------
pub async fn list_posts(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Query(query): Query<PostListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PostRead)
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
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PostRead)
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

pub async fn create_post(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Json(payload): Json<PostCreateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;

    let actor = super::state::ApiState::actor_label(&principal);

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
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PostUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

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
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PostStatusRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

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

pub async fn replace_post_tags(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PostTagsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

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
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PostWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

    state
        .posts
        .delete_post(&actor, id)
        .await
        .map_err(post_to_api)?;

    Ok(StatusCode::NO_CONTENT)
}

/// -------- Pages --------
pub async fn list_pages(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Query(query): Query<PageListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PageRead)
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
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PageRead)
        .map_err(|_| ApiError::forbidden())?;

    let page = state.pages.find_by_slug(&slug).await.map_err(page_to_api)?;

    match page {
        Some(page) => Ok(Json(page)),
        None => Err(ApiError::not_found("page not found")),
    }
}

pub async fn create_page(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Json(payload): Json<PageCreateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PageWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

    let command = CreatePageCommand {
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
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PageUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PageWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

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

pub async fn update_page_status(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PageStatusRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PageWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

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
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::PageWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

    state
        .pages
        .delete_page(&actor, id)
        .await
        .map_err(page_to_api)?;

    Ok(StatusCode::NO_CONTENT)
}

/// -------- Tags --------
pub async fn list_tags(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Query(query): Query<TagListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::TagRead)
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

pub async fn create_tag(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Json(payload): Json<TagCreateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::TagWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

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
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<TagUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::TagWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

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

pub async fn delete_tag(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::TagWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

    state
        .tags
        .delete_tag(&actor, id)
        .await
        .map_err(tag_to_api)?;

    Ok(StatusCode::NO_CONTENT)
}

/// -------- Navigation --------
pub async fn list_navigation(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Query(query): Query<NavigationListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::NavigationRead)
        .map_err(|_| ApiError::forbidden())?;
    let settings = state.settings.load().await.map_err(settings_to_api)?;
    let limit = query
        .limit
        .unwrap_or_else(|| settings.admin_page_size.max(1) as u32)
        .clamp(1, 100);

    let cursor = match query
        .cursor
        .as_deref()
        .map(NavigationCursor::decode)
        .transpose()
    {
        Ok(cursor) => cursor,
        Err(err) => {
            return Err(ApiError::bad_request(
                "invalid cursor",
                Some(err.to_string()),
            ));
        }
    };

    let filter = NavigationQueryFilter {
        search: query.search,
    };

    let page = state
        .navigation
        .list(query.visible, &filter, PageRequest::new(limit, cursor))
        .await
        .map_err(nav_to_api)?;

    Ok(Json(page))
}

pub async fn create_navigation(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Json(payload): Json<NavigationCreateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::NavigationWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

    let command = CreateNavigationItemCommand {
        label: payload.label,
        destination_type: payload.destination_type,
        destination_page_id: payload.destination_page_id,
        destination_url: payload.destination_url,
        sort_order: payload.sort_order,
        visible: payload.visible,
        open_in_new_tab: payload.open_in_new_tab,
    };

    let record = state
        .navigation
        .create_item(&actor, command)
        .await
        .map_err(nav_to_api)?;

    Ok((StatusCode::CREATED, Json(record)))
}

pub async fn update_navigation(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NavigationUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::NavigationWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

    let command = UpdateNavigationItemCommand {
        id,
        label: payload.label,
        destination_type: payload.destination_type,
        destination_page_id: payload.destination_page_id,
        destination_url: payload.destination_url,
        sort_order: payload.sort_order,
        visible: payload.visible,
        open_in_new_tab: payload.open_in_new_tab,
    };

    let record = state
        .navigation
        .update_item(&actor, command)
        .await
        .map_err(nav_to_api)?;

    Ok(Json(record))
}

pub async fn delete_navigation(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::NavigationWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

    state
        .navigation
        .delete_item(&actor, id)
        .await
        .map_err(nav_to_api)?;

    Ok(StatusCode::NO_CONTENT)
}

/// -------- Uploads --------
pub async fn list_uploads(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Query(query): Query<UploadListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::UploadRead)
        .map_err(|_| ApiError::forbidden())?;
    let settings = state.settings.load().await.map_err(settings_to_api)?;
    let limit = query
        .limit
        .unwrap_or_else(|| settings.admin_page_size.max(1) as u32)
        .clamp(1, 100);

    let cursor = match query
        .cursor
        .as_deref()
        .map(UploadCursor::decode)
        .transpose()
    {
        Ok(cursor) => cursor,
        Err(err) => {
            return Err(ApiError::bad_request(
                "invalid cursor",
                Some(err.to_string()),
            ));
        }
    };

    let filter = UploadQueryFilter {
        content_type: query.content_type,
        month: query.month,
        search: query.search,
    };

    let page = state
        .uploads
        .list(&filter, PageRequest::new(limit, cursor))
        .await
        .map_err(upload_to_api)?;

    Ok(Json(page))
}

pub async fn upload_file(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::UploadWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

    let mut filename = None;
    let mut content_type = None;
    let mut data: Option<bytes::Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| ApiError::bad_request("invalid multipart payload", Some(err.to_string())))?
    {
        if field.name() == Some("file") {
            filename = field.file_name().map(|s| s.to_string());
            content_type = field.content_type().map(|s| s.to_string());
            data = Some(field.bytes().await.map_err(|err| {
                ApiError::bad_request("failed to read upload", Some(err.to_string()))
            })?);
            break;
        }
    }

    let filename = filename.ok_or_else(|| ApiError::bad_request("missing file", None))?;
    let data = data.ok_or_else(|| ApiError::bad_request("missing file", None))?;
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let stored = state
        .upload_storage
        .store(&filename, data)
        .await
        .map_err(upload_storage_to_api)?;

    let record = UploadRecord {
        id: Uuid::new_v4(),
        filename: filename.clone(),
        content_type: content_type.clone(),
        size_bytes: stored.size_bytes,
        checksum: stored.checksum.clone(),
        stored_path: stored.stored_path.clone(),
        metadata: crate::domain::uploads::UploadMetadata::default(),
        created_at: OffsetDateTime::now_utc(),
    };

    state
        .uploads
        .register_upload(&actor, record.clone())
        .await
        .map_err(upload_to_api)?;

    let response = UploadResponse {
        id: record.id,
        filename: record.filename,
        content_type: record.content_type,
        size_bytes: record.size_bytes,
        checksum: record.checksum,
        stored_path: record.stored_path,
        created_at: record.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete_upload(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::UploadWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

    let record = state
        .uploads
        .delete_upload(&actor, id)
        .await
        .map_err(upload_to_api)?;

    let _ = state.upload_storage.delete(&record.stored_path).await;

    Ok(StatusCode::NO_CONTENT)
}

/// -------- Settings --------
pub async fn get_settings(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::SettingsRead)
        .map_err(|_| ApiError::forbidden())?;

    let settings = state.settings.load().await.map_err(settings_to_api)?;
    Ok(Json(settings))
}

pub async fn patch_settings(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Json(payload): Json<SettingsPatchRequest>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::SettingsWrite)
        .map_err(|_| ApiError::forbidden())?;
    let actor = super::state::ApiState::actor_label(&principal);

    let mut current = state.settings.load().await.map_err(settings_to_api)?;

    if let Some(val) = payload.brand_title {
        current.brand_title = val;
    }
    if let Some(val) = payload.brand_href {
        current.brand_href = val;
    }
    if let Some(val) = payload.footer_copy {
        current.footer_copy = val;
    }
    if let Some(val) = payload.homepage_size {
        current.homepage_size = val;
    }
    if let Some(val) = payload.admin_page_size {
        current.admin_page_size = val;
    }
    if let Some(val) = payload.show_tag_aggregations {
        current.show_tag_aggregations = val;
    }
    if let Some(val) = payload.show_month_aggregations {
        current.show_month_aggregations = val;
    }
    if let Some(val) = payload.tag_filter_limit {
        current.tag_filter_limit = val;
    }
    if let Some(val) = payload.month_filter_limit {
        current.month_filter_limit = val;
    }
    if let Some(val) = payload.timezone {
        current.timezone = val
            .parse::<chrono_tz::Tz>()
            .map_err(|err| ApiError::bad_request("invalid timezone", Some(err.to_string())))?;
    }
    if let Some(val) = payload.meta_title {
        current.meta_title = val;
    }
    if let Some(val) = payload.meta_description {
        current.meta_description = val;
    }
    if let Some(val) = payload.og_title {
        current.og_title = val;
    }
    if let Some(val) = payload.og_description {
        current.og_description = val;
    }
    if let Some(val) = payload.public_site_url {
        current.public_site_url = val;
    }

    let command = crate::application::admin::settings::UpdateSettingsCommand {
        homepage_size: current.homepage_size,
        admin_page_size: current.admin_page_size,
        show_tag_aggregations: current.show_tag_aggregations,
        show_month_aggregations: current.show_month_aggregations,
        tag_filter_limit: current.tag_filter_limit,
        month_filter_limit: current.month_filter_limit,
        global_toc_enabled: current.global_toc_enabled,
        brand_title: current.brand_title.clone(),
        brand_href: current.brand_href.clone(),
        footer_copy: current.footer_copy.clone(),
        public_site_url: current.public_site_url.clone(),
        favicon_svg: current.favicon_svg.clone(),
        timezone: current.timezone,
        meta_title: current.meta_title.clone(),
        meta_description: current.meta_description.clone(),
        og_title: current.og_title.clone(),
        og_description: current.og_description.clone(),
    };

    let updated = state
        .settings
        .update(&actor, command)
        .await
        .map_err(settings_to_api)?;

    Ok(Json(updated))
}

/// -------- Jobs & Audit (read-only) --------
pub async fn list_jobs(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Query(query): Query<JobsListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::JobRead)
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

pub async fn list_audit_logs(
    State(state): State<ApiState>,
    Extension(principal): Extension<crate::application::api_keys::ApiPrincipal>,
    Query(query): Query<AuditListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    principal
        .requires(crate::domain::api_keys::ApiScope::AuditRead)
        .map_err(|_| ApiError::forbidden())?;

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let cursor = match query
        .cursor
        .as_deref()
        .map(crate::application::pagination::AuditCursor::decode)
        .transpose()
    {
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

/// -------- Helper conversions --------
fn repo_to_api(err: RepoError) -> ApiError {
    match err {
        RepoError::Duplicate { constraint } => ApiError::new(
            StatusCode::CONFLICT,
            codes::DUPLICATE,
            "Duplicate record",
            Some(constraint),
        ),
        RepoError::Pagination(p) => ApiError::new(
            StatusCode::BAD_REQUEST,
            codes::INVALID_CURSOR,
            "Invalid cursor",
            Some(p.to_string()),
        ),
        RepoError::NotFound => ApiError::not_found("resource not found"),
        RepoError::InvalidInput { message } => ApiError::new(
            StatusCode::BAD_REQUEST,
            codes::INVALID_INPUT,
            "Invalid input",
            Some(message),
        ),
        RepoError::Integrity { message } => ApiError::new(
            StatusCode::CONFLICT,
            codes::INTEGRITY,
            "Integrity constraint violated",
            Some(message),
        ),
        RepoError::Timeout => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            codes::DB_TIMEOUT,
            "Database timeout",
            None,
        ),
        RepoError::Persistence(msg) => ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            codes::REPO,
            "Persistence error",
            Some(msg),
        ),
    }
}

fn post_to_api(err: AdminPostError) -> ApiError {
    match err {
        AdminPostError::ConstraintViolation(field) => ApiError::new(
            StatusCode::BAD_REQUEST,
            codes::INVALID_INPUT,
            "Invalid post",
            Some(field.to_string()),
        ),
        AdminPostError::Repo(repo) => repo_to_api(repo),
    }
}

fn page_to_api(err: AdminPageError) -> ApiError {
    match err {
        AdminPageError::ConstraintViolation(field) => ApiError::new(
            StatusCode::BAD_REQUEST,
            codes::INVALID_INPUT,
            "Invalid page",
            Some(field.to_string()),
        ),
        AdminPageError::Render(render_err) => ApiError::new(
            StatusCode::BAD_REQUEST,
            codes::RENDER,
            "Rendering failed",
            Some(render_err.to_string()),
        ),
        AdminPageError::Repo(repo) => repo_to_api(repo),
    }
}

fn tag_to_api(err: AdminTagError) -> ApiError {
    match err {
        AdminTagError::ConstraintViolation(field) => ApiError::new(
            StatusCode::BAD_REQUEST,
            codes::INVALID_INPUT,
            "Invalid tag",
            Some(field.to_string()),
        ),
        AdminTagError::Repo(repo) => repo_to_api(repo),
        AdminTagError::InUse { .. } => ApiError::new(
            StatusCode::BAD_REQUEST,
            codes::TAG_IN_USE,
            "Tag is in use",
            None,
        ),
    }
}

fn nav_to_api(err: AdminNavigationError) -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        codes::NAVIGATION,
        "Navigation update failed",
        Some(err.to_string()),
    )
}

fn upload_to_api(err: AdminUploadError) -> ApiError {
    match err {
        AdminUploadError::NotFound => ApiError::not_found("upload not found"),
        AdminUploadError::Repo(repo) => repo_to_api(repo),
    }
}

fn upload_storage_to_api(err: UploadStorageError) -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        codes::UPLOAD,
        "Failed to store upload",
        Some(err.to_string()),
    )
}

fn settings_to_api(err: AdminSettingsError) -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        codes::SETTINGS,
        "Settings update failed",
        Some(err.to_string()),
    )
}
