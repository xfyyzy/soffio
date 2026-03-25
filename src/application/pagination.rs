//! Shared cursor pagination helpers.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::types::PostStatus;

#[path = "pagination/api_key.rs"]
mod api_key;
#[path = "pagination/audit.rs"]
mod audit;
#[path = "pagination/job.rs"]
mod job;
#[path = "pagination/navigation.rs"]
mod navigation;
#[path = "pagination/page.rs"]
mod page;
#[path = "pagination/post.rs"]
mod post;
#[path = "pagination/snapshot.rs"]
mod snapshot;
#[path = "pagination/tag.rs"]
mod tag;
#[path = "pagination/upload.rs"]
mod upload;

pub use api_key::ApiKeyCursor;
pub use audit::AuditCursor;
pub use job::JobCursor;
pub use navigation::NavigationCursor;
pub use page::PageCursor;
pub use post::PostCursor;
pub use snapshot::SnapshotCursor;
pub use tag::TagCursor;
pub use upload::UploadCursor;
/// Cursor-aware pagination request.
#[derive(Debug, Clone, Copy)]
pub struct PageRequest<C> {
    pub limit: u32,
    pub cursor: Option<C>,
}

impl<C> PageRequest<C> {
    pub fn new(limit: u32, cursor: Option<C>) -> Self {
        Self { limit, cursor }
    }
}

/// Cursor-aware page result.
#[derive(Debug, Clone, Serialize)]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

impl<T> CursorPage<T> {
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            next_cursor: None,
        }
    }

    pub fn new(items: Vec<T>, next_cursor: Option<String>) -> Self {
        Self { items, next_cursor }
    }
}

#[derive(Debug, Error)]
pub enum PaginationError {
    #[error("invalid cursor: {0}")]
    InvalidCursor(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_cursor_round_trip_public() {
        let id = Uuid::new_v4();
        let when = OffsetDateTime::now_utc();
        let cursor = PostCursor::published(when, id, true);
        let encoded = cursor.encode();
        let decoded = PostCursor::decode(&encoded).expect("decoded cursor");

        assert_eq!(decoded.id(), id);
        assert_eq!(decoded.sort_key(), when);
        assert_eq!(decoded.status(), Some(PostStatus::Published));
        assert!(decoded.pinned());
    }

    #[test]
    fn page_cursor_round_trip() {
        let id = Uuid::new_v4();
        let primary_time = OffsetDateTime::now_utc();
        let cursor = PageCursor::new(primary_time, id);
        let encoded = cursor.encode();
        let decoded = PageCursor::decode(&encoded).expect("decoded page cursor");

        assert_eq!(decoded.primary_time(), primary_time);
        assert_eq!(decoded.id(), id);
    }

    #[test]
    fn tag_cursor_round_trip() {
        let id = Uuid::new_v4();
        let primary_time = OffsetDateTime::now_utc();
        let cursor = TagCursor::new(true, primary_time, id);
        let encoded = cursor.encode();
        let decoded = TagCursor::decode(&encoded).expect("decoded tag cursor");

        assert!(decoded.pinned());
        assert_eq!(decoded.primary_time(), primary_time);
        assert_eq!(decoded.id(), id);
    }

    #[test]
    fn upload_cursor_round_trip() {
        let id = Uuid::new_v4();
        let created_at = OffsetDateTime::now_utc();
        let cursor = UploadCursor::new(created_at, id);
        let encoded = cursor.encode();
        let decoded = UploadCursor::decode(&encoded).expect("decoded upload cursor");

        assert_eq!(decoded.id(), id);
        assert_eq!(decoded.created_at(), created_at);
    }

    #[test]
    fn navigation_cursor_round_trip() {
        let id = Uuid::new_v4();
        let primary_time = OffsetDateTime::now_utc();
        let cursor = NavigationCursor::new(10, primary_time, id);
        let encoded = cursor.encode();
        let decoded = NavigationCursor::decode(&encoded).expect("decoded navigation cursor");

        assert_eq!(decoded.sort_order(), 10);
        assert_eq!(decoded.primary_time(), primary_time);
        assert_eq!(decoded.id(), id);
    }

    #[test]
    fn job_cursor_round_trip() {
        let when = OffsetDateTime::now_utc();
        let cursor = JobCursor::new(when, "job-1");
        let encoded = cursor.encode();
        let decoded = JobCursor::decode(&encoded).expect("decoded job cursor");

        assert_eq!(decoded.run_at(), when);
        assert_eq!(decoded.id(), "job-1");
    }

    #[test]
    fn audit_cursor_round_trip() {
        let id = Uuid::new_v4();
        let when = OffsetDateTime::now_utc();
        let cursor = AuditCursor::new(when, id);
        let encoded = cursor.encode();
        let decoded = AuditCursor::decode(&encoded).expect("decoded audit cursor");

        assert_eq!(decoded.created_at(), when);
        assert_eq!(decoded.id(), id);
    }

    #[test]
    fn snapshot_cursor_round_trip() {
        let id = Uuid::new_v4();
        let when = OffsetDateTime::now_utc();
        let cursor = SnapshotCursor::new(when, id);
        let encoded = cursor.encode();
        let decoded = SnapshotCursor::decode(&encoded).expect("decoded snapshot cursor");

        assert_eq!(decoded.created_at(), when);
        assert_eq!(decoded.id(), id);
    }

    #[test]
    fn decoding_invalid_cursor_reports_error() {
        let err = PostCursor::decode("not-base64").expect_err("invalid cursor rejected");
        assert!(matches!(err, PaginationError::InvalidCursor(_)));
    }
}
