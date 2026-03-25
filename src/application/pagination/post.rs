use super::*;

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

/// Cursor for paginating posts in public or administrative contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostCursor {
    scope: CursorScope,
    status: Option<PostStatus>,
    sort_key: OffsetDateTime,
    id: Uuid,
    pinned: bool,
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
