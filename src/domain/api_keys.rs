//! Domain types for API keys and scopes.

use serde::{Deserialize, Serialize};
pub use soffio_api_types::{ApiKeyStatus, ApiScope};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub prefix: String,
    pub hashed_secret: Vec<u8>,
    pub scopes: Vec<ApiScope>,
    pub status: ApiKeyStatus,
    pub expires_in: Option<time::Duration>,
    pub expires_at: Option<OffsetDateTime>,
    pub revoked_at: Option<OffsetDateTime>,
    pub last_used_at: Option<OffsetDateTime>,
    pub created_by: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl ApiKeyRecord {
    /// Checks if this key is currently active (not revoked, not expired).
    /// This is a runtime check that accounts for keys that may have expired
    /// between cron runs.
    pub fn is_active_at(&self, now: OffsetDateTime) -> bool {
        if self.status != ApiKeyStatus::Active {
            return false;
        }
        // Fallback check for keys that expired between cron runs
        if let Some(expires_at) = self.expires_at {
            return expires_at > now;
        }
        true
    }

    pub fn is_revoked(&self) -> bool {
        self.status == ApiKeyStatus::Revoked
    }

    pub fn is_expired(&self) -> bool {
        self.status == ApiKeyStatus::Expired
    }
}
