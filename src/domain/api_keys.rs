//! Domain types for API keys and scopes.

use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

/// Status of an API key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "api_key_status", rename_all = "snake_case")]
pub enum ApiKeyStatus {
    Active,
    Revoked,
    Expired,
}

impl ApiKeyStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Revoked => "Revoked",
            Self::Expired => "Expired",
        }
    }
}

impl Display for ApiKeyStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ApiKeyStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "revoked" => Ok(Self::Revoked),
            "expired" => Ok(Self::Expired),
            _ => Err(()),
        }
    }
}

/// API permission scope with domain/action granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "api_scope", rename_all = "snake_case")]
pub enum ApiScope {
    PostRead,
    PostWrite,
    PageRead,
    PageWrite,
    TagRead,
    TagWrite,
    NavigationRead,
    NavigationWrite,
    UploadRead,
    UploadWrite,
    SettingsRead,
    SettingsWrite,
    JobRead,
    AuditRead,
}

impl ApiScope {
    /// Returns the slug used for serialization and DB storage.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PostRead => "post_read",
            Self::PostWrite => "post_write",
            Self::PageRead => "page_read",
            Self::PageWrite => "page_write",
            Self::TagRead => "tag_read",
            Self::TagWrite => "tag_write",
            Self::NavigationRead => "navigation_read",
            Self::NavigationWrite => "navigation_write",
            Self::UploadRead => "upload_read",
            Self::UploadWrite => "upload_write",
            Self::SettingsRead => "settings_read",
            Self::SettingsWrite => "settings_write",
            Self::JobRead => "job_read",
            Self::AuditRead => "audit_read",
        }
    }

    /// Returns the human-readable display name for UI.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::PostRead => "Post read",
            Self::PostWrite => "Post write",
            Self::PageRead => "Page read",
            Self::PageWrite => "Page write",
            Self::TagRead => "Tag read",
            Self::TagWrite => "Tag write",
            Self::NavigationRead => "Navigation read",
            Self::NavigationWrite => "Navigation write",
            Self::UploadRead => "Upload read",
            Self::UploadWrite => "Upload write",
            Self::SettingsRead => "Settings read",
            Self::SettingsWrite => "Settings write",
            Self::JobRead => "Job read",
            Self::AuditRead => "Audit read",
        }
    }

    /// Returns all scope variants for iteration.
    pub fn all() -> &'static [ApiScope] {
        &[
            Self::PostRead,
            Self::PostWrite,
            Self::PageRead,
            Self::PageWrite,
            Self::TagRead,
            Self::TagWrite,
            Self::NavigationRead,
            Self::NavigationWrite,
            Self::UploadRead,
            Self::UploadWrite,
            Self::SettingsRead,
            Self::SettingsWrite,
            Self::JobRead,
            Self::AuditRead,
        ]
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
            "post_read" => Ok(Self::PostRead),
            "post_write" => Ok(Self::PostWrite),
            "page_read" => Ok(Self::PageRead),
            "page_write" => Ok(Self::PageWrite),
            "tag_read" => Ok(Self::TagRead),
            "tag_write" => Ok(Self::TagWrite),
            "navigation_read" => Ok(Self::NavigationRead),
            "navigation_write" => Ok(Self::NavigationWrite),
            "upload_read" => Ok(Self::UploadRead),
            "upload_write" => Ok(Self::UploadWrite),
            "settings_read" => Ok(Self::SettingsRead),
            "settings_write" => Ok(Self::SettingsWrite),
            "job_read" => Ok(Self::JobRead),
            "audit_read" => Ok(Self::AuditRead),
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
