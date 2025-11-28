//! Domain types for API keys and scopes.

use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "api_scope", rename_all = "snake_case")]
pub enum ApiScope {
    ContentRead,
    ContentWrite,
    TagWrite,
    NavigationWrite,
    UploadWrite,
    SettingsWrite,
    JobsRead,
    AuditRead,
}

impl ApiScope {
    pub fn as_str(self) -> &'static str {
        match self {
            ApiScope::ContentRead => "content_read",
            ApiScope::ContentWrite => "content_write",
            ApiScope::TagWrite => "tag_write",
            ApiScope::NavigationWrite => "navigation_write",
            ApiScope::UploadWrite => "upload_write",
            ApiScope::SettingsWrite => "settings_write",
            ApiScope::JobsRead => "jobs_read",
            ApiScope::AuditRead => "audit_read",
        }
    }
}

impl Display for ApiScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ApiScope {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "content_read" => Ok(ApiScope::ContentRead),
            "content_write" => Ok(ApiScope::ContentWrite),
            "tag_write" => Ok(ApiScope::TagWrite),
            "navigation_write" => Ok(ApiScope::NavigationWrite),
            "upload_write" => Ok(ApiScope::UploadWrite),
            "settings_write" => Ok(ApiScope::SettingsWrite),
            "jobs_read" => Ok(ApiScope::JobsRead),
            "audit_read" => Ok(ApiScope::AuditRead),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub prefix: String,
    pub hashed_secret: Vec<u8>,
    pub scopes: Vec<ApiScope>,
    pub expires_at: Option<OffsetDateTime>,
    pub revoked_at: Option<OffsetDateTime>,
    pub last_used_at: Option<OffsetDateTime>,
    pub created_by: String,
    pub created_at: OffsetDateTime,
}

impl ApiKeyRecord {
    pub fn is_active_at(&self, now: OffsetDateTime) -> bool {
        if let Some(revoked_at) = self.revoked_at {
            return revoked_at > now;
        }
        if let Some(expires_at) = self.expires_at {
            return expires_at > now;
        }
        true
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
}
