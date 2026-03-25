use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobCursorPayload {
    run_at: OffsetDateTime,
    id: String,
}

/// Cursor for paginating jobs in reverse chronological order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobCursor {
    run_at: OffsetDateTime,
    id: String,
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
