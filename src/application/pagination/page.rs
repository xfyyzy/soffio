use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PageCursorPayload {
    primary_time: OffsetDateTime,
    id: Uuid,
}

/// Cursor for paginating static pages based on their primary time ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageCursor {
    primary_time: OffsetDateTime,
    id: Uuid,
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
