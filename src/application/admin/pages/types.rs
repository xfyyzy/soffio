use serde::Serialize;
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{render::RenderError, repos::RepoError},
    domain::types::PageStatus,
};

#[derive(Debug, Error)]
pub enum AdminPageError {
    #[error("{0}")]
    ConstraintViolation(&'static str),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error(transparent)]
    Repo(#[from] RepoError),
}

#[derive(Debug, Clone)]
pub struct CreatePageCommand {
    pub slug: Option<String>,
    pub title: String,
    pub body_markdown: String,
    pub status: PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
pub struct UpdatePageContentCommand {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub body_markdown: String,
}

#[derive(Debug, Clone)]
pub struct UpdatePageStatusCommand {
    pub id: Uuid,
    pub status: PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminPageStatusCounts {
    pub total: u64,
    pub draft: u64,
    pub published: u64,
    pub archived: u64,
    pub error: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageSummarySnapshot<'a> {
    pub slug: &'a str,
    pub title: &'a str,
    pub status: PageStatus,
}

pub(super) struct StatusTimestamps {
    pub(super) scheduled_at: Option<OffsetDateTime>,
    pub(super) published_at: Option<OffsetDateTime>,
    pub(super) archived_at: Option<OffsetDateTime>,
}

pub(super) fn normalize_status(
    status: PageStatus,
    scheduled_at: Option<OffsetDateTime>,
    published_at: Option<OffsetDateTime>,
    archived_at: Option<OffsetDateTime>,
) -> Result<StatusTimestamps, AdminPageError> {
    match status {
        PageStatus::Published => Ok(StatusTimestamps {
            scheduled_at: None,
            published_at: Some(published_at.unwrap_or_else(OffsetDateTime::now_utc)),
            archived_at: None,
        }),
        PageStatus::Archived => Ok(StatusTimestamps {
            scheduled_at: None,
            published_at,
            archived_at: Some(archived_at.unwrap_or_else(OffsetDateTime::now_utc)),
        }),
        PageStatus::Draft => Ok(StatusTimestamps {
            scheduled_at,
            published_at: None,
            archived_at: None,
        }),
        PageStatus::Error => Ok(StatusTimestamps {
            scheduled_at,
            published_at,
            archived_at,
        }),
    }
}

pub(super) fn ensure_non_empty(value: &str, field: &'static str) -> Result<(), AdminPageError> {
    if value.trim().is_empty() {
        return Err(AdminPageError::ConstraintViolation(field));
    }
    Ok(())
}

pub(super) fn normalize_public_site_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let without_trailing = trimmed.trim_end_matches('/');
    format!("{without_trailing}/")
}
