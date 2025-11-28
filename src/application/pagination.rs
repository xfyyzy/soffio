//! Shared cursor pagination helpers.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::types::PostStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CursorScope {
    Public,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PostCursorPayload {
    scope: CursorScope,
    status: Option<PostStatus>,
    sort_key: OffsetDateTime,
    id: Uuid,
    #[serde(default)]
    pinned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PageCursorPayload {
    primary_time: OffsetDateTime,
    id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct NavigationCursorPayload {
    sort_order: i32,
    primary_time: OffsetDateTime,
    id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct TagCursorPayload {
    pinned: bool,
    primary_time: OffsetDateTime,
    id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobCursorPayload {
    run_at: OffsetDateTime,
    id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct AuditCursorPayload {
    created_at: OffsetDateTime,
    id: Uuid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct UploadCursorPayload {
    created_at: OffsetDateTime,
    id: Uuid,
}

/// Cursor for paginating posts in public or administrative contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostCursor {
    scope: CursorScope,
    status: Option<PostStatus>,
    sort_key: OffsetDateTime,
    id: Uuid,
    pinned: bool,
}

/// Cursor for paginating static pages based on their primary time ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageCursor {
    primary_time: OffsetDateTime,
    id: Uuid,
}

/// Cursor for paginating navigation entries ordered by manual order then recency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationCursor {
    sort_order: i32,
    primary_time: OffsetDateTime,
    id: Uuid,
}

/// Cursor for paginating tags in administrative contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagCursor {
    pinned: bool,
    primary_time: OffsetDateTime,
    id: Uuid,
}

/// Cursor for paginating jobs in reverse chronological order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobCursor {
    run_at: OffsetDateTime,
    id: String,
}

/// Cursor for paginating audit log entries in reverse chronological order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditCursor {
    created_at: OffsetDateTime,
    id: Uuid,
}

/// Cursor for paginating uploads in reverse chronological order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadCursor {
    created_at: OffsetDateTime,
    id: Uuid,
}

impl PostCursor {
    /// Construct a cursor for public listings (published posts only).
    pub fn published(sort_key: OffsetDateTime, id: Uuid, pinned: bool) -> Self {
        Self {
            scope: CursorScope::Public,
            status: Some(PostStatus::Published),
            sort_key,
            id,
            pinned,
        }
    }

    /// Construct a cursor for administrative listings scoped to a status filter.
    pub fn admin(status: PostStatus, sort_key: OffsetDateTime, id: Uuid, pinned: bool) -> Self {
        Self {
            scope: CursorScope::Admin,
            status: Some(status),
            sort_key,
            id,
            pinned,
        }
    }

    pub fn status(&self) -> Option<PostStatus> {
        self.status
    }

    pub fn sort_key(&self) -> OffsetDateTime {
        self.sort_key
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn pinned(&self) -> bool {
        self.pinned
    }

    pub fn encode(&self) -> String {
        let payload = PostCursorPayload {
            scope: self.scope,
            status: self.status,
            sort_key: self.sort_key,
            id: self.id,
            pinned: self.pinned,
        };
        let serialized =
            serde_json::to_vec(&payload).expect("serializing post cursor payload should succeed");
        URL_SAFE_NO_PAD.encode(serialized)
    }

    pub fn decode(cursor: &str) -> Result<Self, PaginationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        let payload: PostCursorPayload = serde_json::from_slice(&bytes)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        Ok(Self {
            scope: payload.scope,
            status: payload.status,
            sort_key: payload.sort_key,
            id: payload.id,
            pinned: payload.pinned,
        })
    }
}

impl PageCursor {
    pub fn new(primary_time: OffsetDateTime, id: Uuid) -> Self {
        Self { primary_time, id }
    }

    pub fn primary_time(&self) -> OffsetDateTime {
        self.primary_time
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn encode(&self) -> String {
        let payload = PageCursorPayload {
            primary_time: self.primary_time,
            id: self.id,
        };
        let serialized =
            serde_json::to_vec(&payload).expect("serializing page cursor payload should succeed");
        URL_SAFE_NO_PAD.encode(serialized)
    }

    pub fn decode(cursor: &str) -> Result<Self, PaginationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        let payload: PageCursorPayload = serde_json::from_slice(&bytes)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        Ok(Self {
            primary_time: payload.primary_time,
            id: payload.id,
        })
    }
}

impl NavigationCursor {
    pub fn new(sort_order: i32, primary_time: OffsetDateTime, id: Uuid) -> Self {
        Self {
            sort_order,
            primary_time,
            id,
        }
    }

    pub fn sort_order(&self) -> i32 {
        self.sort_order
    }

    pub fn primary_time(&self) -> OffsetDateTime {
        self.primary_time
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn encode(&self) -> String {
        let payload = NavigationCursorPayload {
            sort_order: self.sort_order,
            primary_time: self.primary_time,
            id: self.id,
        };
        let serialized = serde_json::to_vec(&payload)
            .expect("serializing navigation cursor payload should succeed");
        URL_SAFE_NO_PAD.encode(serialized)
    }

    pub fn decode(cursor: &str) -> Result<Self, PaginationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        let payload: NavigationCursorPayload = serde_json::from_slice(&bytes)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        Ok(Self {
            sort_order: payload.sort_order,
            primary_time: payload.primary_time,
            id: payload.id,
        })
    }
}

impl TagCursor {
    /// Construct a cursor from a tag row.
    pub fn new(pinned: bool, primary_time: OffsetDateTime, id: Uuid) -> Self {
        Self {
            pinned,
            primary_time,
            id,
        }
    }

    pub fn pinned(&self) -> bool {
        self.pinned
    }

    pub fn primary_time(&self) -> OffsetDateTime {
        self.primary_time
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn encode(&self) -> String {
        let payload = TagCursorPayload {
            pinned: self.pinned,
            primary_time: self.primary_time,
            id: self.id,
        };
        let serialized =
            serde_json::to_vec(&payload).expect("serializing tag cursor payload should succeed");
        URL_SAFE_NO_PAD.encode(serialized)
    }

    pub fn decode(cursor: &str) -> Result<Self, PaginationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        let payload: TagCursorPayload = serde_json::from_slice(&bytes)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        Ok(Self {
            pinned: payload.pinned,
            primary_time: payload.primary_time,
            id: payload.id,
        })
    }
}

impl JobCursor {
    pub fn new(run_at: OffsetDateTime, id: impl Into<String>) -> Self {
        Self {
            run_at,
            id: id.into(),
        }
    }

    pub fn run_at(&self) -> OffsetDateTime {
        self.run_at
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn encode(&self) -> String {
        let payload = JobCursorPayload {
            run_at: self.run_at,
            id: self.id.clone(),
        };
        let serialized =
            serde_json::to_vec(&payload).expect("serializing job cursor payload should succeed");
        URL_SAFE_NO_PAD.encode(serialized)
    }

    pub fn decode(cursor: &str) -> Result<Self, PaginationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        let payload: JobCursorPayload = serde_json::from_slice(&bytes)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        Ok(Self {
            run_at: payload.run_at,
            id: payload.id,
        })
    }
}

impl AuditCursor {
    pub fn new(created_at: OffsetDateTime, id: Uuid) -> Self {
        Self { created_at, id }
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn encode(&self) -> String {
        let payload = AuditCursorPayload {
            created_at: self.created_at,
            id: self.id,
        };
        let serialized =
            serde_json::to_vec(&payload).expect("serializing audit cursor payload should succeed");
        URL_SAFE_NO_PAD.encode(serialized)
    }

    pub fn decode(cursor: &str) -> Result<Self, PaginationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        let payload: AuditCursorPayload = serde_json::from_slice(&bytes)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        Ok(Self {
            created_at: payload.created_at,
            id: payload.id,
        })
    }
}

impl UploadCursor {
    pub fn new(created_at: OffsetDateTime, id: Uuid) -> Self {
        Self { created_at, id }
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn encode(&self) -> String {
        let payload = UploadCursorPayload {
            created_at: self.created_at,
            id: self.id,
        };
        let serialized =
            serde_json::to_vec(&payload).expect("serializing upload cursor payload should succeed");
        URL_SAFE_NO_PAD.encode(serialized)
    }

    pub fn decode(cursor: &str) -> Result<Self, PaginationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        let payload: UploadCursorPayload = serde_json::from_slice(&bytes)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        Ok(Self {
            created_at: payload.created_at,
            id: payload.id,
        })
    }
}

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
    fn decoding_invalid_cursor_reports_error() {
        let err = PostCursor::decode("not-base64").expect_err("invalid cursor rejected");
        assert!(matches!(err, PaginationError::InvalidCursor(_)));
    }
}
