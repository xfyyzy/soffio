use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct NavigationCursorPayload {
    sort_order: i32,
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
