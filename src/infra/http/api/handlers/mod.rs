//! API handlers organized by resource type.
//!
//! Each submodule contains handlers for a specific resource (posts, pages, etc.).
//! Helper functions for error conversion are defined here and shared across modules.

mod api_keys;
mod audit;
mod jobs;
mod navigation;
mod pages;
mod posts;
mod settings;
mod snapshots;
mod tags;
mod uploads;

// Re-export all handlers for external use
pub use api_keys::*;
pub use audit::*;
pub use jobs::*;
pub use navigation::*;
pub use pages::*;
pub use posts::*;
pub use settings::*;
pub use snapshots::*;
pub use tags::*;
pub use uploads::*;

// ----- Shared query structs -----

use serde::Deserialize;

use crate::domain::types::{JobState, JobType, PageStatus, PostStatus};

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

// ----- Shared error conversions -----

use axum::http::StatusCode;

use crate::application::admin::navigation::AdminNavigationError;
use crate::application::admin::pages::AdminPageError;
use crate::application::admin::posts::AdminPostError;
use crate::application::admin::settings::AdminSettingsError;
use crate::application::admin::snapshots::SnapshotServiceError;
use crate::application::admin::tags::AdminTagError;
use crate::application::admin::uploads::AdminUploadError;
use crate::application::api_keys::ApiKeyError;
use crate::application::repos::RepoError;
use crate::infra::uploads::UploadStorageError;

use super::error::{ApiError, codes};

pub(crate) fn repo_to_api(err: RepoError) -> ApiError {
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

pub(crate) fn post_to_api(err: AdminPostError) -> ApiError {
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

pub(crate) fn page_to_api(err: AdminPageError) -> ApiError {
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

pub(crate) fn tag_to_api(err: AdminTagError) -> ApiError {
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

pub(crate) fn nav_to_api(err: AdminNavigationError) -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        codes::NAVIGATION,
        "Navigation update failed",
        Some(err.to_string()),
    )
}

pub(crate) fn upload_to_api(err: AdminUploadError) -> ApiError {
    match err {
        AdminUploadError::NotFound => ApiError::not_found("upload not found"),
        AdminUploadError::Repo(repo) => repo_to_api(repo),
    }
}

pub(crate) fn upload_storage_to_api(err: UploadStorageError) -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        codes::UPLOAD,
        "Failed to store upload",
        Some(err.to_string()),
    )
}

pub(crate) fn snapshot_to_api(err: SnapshotServiceError) -> ApiError {
    match err {
        SnapshotServiceError::Repo(repo) => repo_to_api(repo),
        SnapshotServiceError::Snapshot(inner) => ApiError::new(
            StatusCode::BAD_REQUEST,
            codes::INVALID_INPUT,
            "Invalid snapshot",
            Some(inner.to_string()),
        ),
        SnapshotServiceError::App(app) => ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            codes::REPO,
            "Snapshot operation failed",
            Some(app.to_string()),
        ),
        SnapshotServiceError::NotFound => ApiError::not_found("snapshot not found"),
    }
}

pub(crate) fn settings_to_api(err: AdminSettingsError) -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        codes::SETTINGS,
        "Settings update failed",
        Some(err.to_string()),
    )
}

pub(crate) fn api_key_to_api(err: ApiKeyError) -> ApiError {
    match err {
        ApiKeyError::InvalidScopes => ApiError::new(
            StatusCode::BAD_REQUEST,
            codes::INVALID_INPUT,
            "Invalid API key scopes",
            None,
        ),
        ApiKeyError::NotFound => ApiError::not_found("api key not found"),
        ApiKeyError::Repo(repo) => repo_to_api(repo),
    }
}
