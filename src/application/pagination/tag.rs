use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct TagCursorPayload {
    pinned: bool,
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
