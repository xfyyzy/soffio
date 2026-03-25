use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct SnapshotCursorPayload {
    created_at: OffsetDateTime,
    id: Uuid,
}

/// Cursor for snapshot pagination (reverse chronological).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotCursor {
    created_at: OffsetDateTime,
    id: Uuid,
}

impl SnapshotCursor {
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
        let payload = SnapshotCursorPayload {
            created_at: self.created_at,
            id: self.id,
        };
        let serialized = serde_json::to_vec(&payload)
            .expect("serializing snapshot cursor payload should succeed");
        URL_SAFE_NO_PAD.encode(serialized)
    }

    pub fn decode(cursor: &str) -> Result<Self, PaginationError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        let payload: SnapshotCursorPayload = serde_json::from_slice(&bytes)
            .map_err(|err| PaginationError::InvalidCursor(err.to_string()))?;
        Ok(Self {
            created_at: payload.created_at,
            id: payload.id,
        })
    }
}
