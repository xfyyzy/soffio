use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct AuditCursorPayload {
    created_at: OffsetDateTime,
    id: Uuid,
}

/// Cursor for paginating audit log entries in reverse chronological order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditCursor {
    created_at: OffsetDateTime,
    id: Uuid,
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
