use serde::Serialize;
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{application::repos::RepoError, domain::types::PostStatus};

#[derive(Debug, Error)]
pub enum AdminPostError {
    #[error("{0}")]
    ConstraintViolation(&'static str),
    #[error(transparent)]
    Repo(#[from] RepoError),
}

#[derive(Debug, Clone, Serialize)]
pub struct PostSummarySnapshot<'a> {
    pub slug: &'a str,
    pub title: &'a str,
    pub status: PostStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostTagsSnapshot<'a> {
    pub slug: &'a str,
    pub title: &'a str,
    pub tags: &'a [String],
}

#[derive(Debug, Clone)]
pub struct CreatePostCommand {
    pub title: String,
    pub excerpt: String,
    pub body_markdown: String,
    pub summary_markdown: Option<String>,
    pub status: PostStatus,
    pub pinned: bool,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
pub struct UpdatePostContentCommand {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub body_markdown: String,
    pub pinned: bool,
    pub summary_markdown: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdatePostStatusCommand {
    pub id: Uuid,
    pub status: PostStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
pub struct AdminPostStatusCounts {
    pub total: u64,
    pub draft: u64,
    pub published: u64,
    pub archived: u64,
    pub error: u64,
}

pub struct StatusTimestamps {
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

pub fn normalize_status(
    status: PostStatus,
    scheduled_at: Option<OffsetDateTime>,
    published_at: Option<OffsetDateTime>,
    archived_at: Option<OffsetDateTime>,
) -> Result<StatusTimestamps, AdminPostError> {
    match status {
        PostStatus::Published => Ok(StatusTimestamps {
            scheduled_at: None,
            published_at: Some(published_at.unwrap_or_else(OffsetDateTime::now_utc)),
            archived_at: None,
        }),
        PostStatus::Archived => Ok(StatusTimestamps {
            scheduled_at: None,
            published_at,
            archived_at: Some(archived_at.unwrap_or_else(OffsetDateTime::now_utc)),
        }),
        PostStatus::Draft => Ok(StatusTimestamps {
            scheduled_at,
            published_at: None,
            archived_at: None,
        }),
        PostStatus::Error => Ok(StatusTimestamps {
            scheduled_at,
            published_at,
            archived_at,
        }),
    }
}

pub fn ensure_non_empty(value: &str, field: &'static str) -> Result<(), AdminPostError> {
    if value.trim().is_empty() {
        return Err(AdminPostError::ConstraintViolation(field));
    }
    Ok(())
}
