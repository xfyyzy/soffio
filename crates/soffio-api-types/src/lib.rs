use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

/// Post publication status persisted in the database and exposed via the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "post_status", rename_all = "snake_case")
)]
pub enum PostStatus {
    Draft,
    Published,
    Archived,
    Error,
}

/// Page publication status persisted in the database and exposed via the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "page_status", rename_all = "snake_case")
)]
pub enum PageStatus {
    Draft,
    Published,
    Archived,
    Error,
}

/// Navigation destination kind persisted in the database and exposed via the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "navigation_destination_type", rename_all = "snake_case")
)]
pub enum NavigationDestinationType {
    Internal,
    External,
}

/// Supported snapshot entity types (mirrors Postgres enum `snapshot_entity_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "snapshot_entity_type", rename_all = "snake_case")
)]
pub enum SnapshotEntityType {
    Post,
    Page,
}

/// Status of an API key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "api_key_status", rename_all = "snake_case")
)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "api_scope", rename_all = "snake_case")
)]
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
    SnapshotRead,
    SnapshotWrite,
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
            Self::SnapshotRead => "snapshot_read",
            Self::SnapshotWrite => "snapshot_write",
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
            Self::SnapshotRead => "Snapshot read",
            Self::SnapshotWrite => "Snapshot write",
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
            Self::SnapshotRead,
            Self::SnapshotWrite,
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
            "snapshot_read" => Ok(Self::SnapshotRead),
            "snapshot_write" => Ok(Self::SnapshotWrite),
            _ => Err(()),
        }
    }
}

fn default_post_status() -> PostStatus {
    PostStatus::Draft
}

fn default_page_status() -> PageStatus {
    PageStatus::Draft
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostCreateRequest {
    pub title: String,
    pub excerpt: String,
    pub body_markdown: String,
    pub summary_markdown: Option<String>,
    #[serde(default = "default_post_status")]
    pub status: PostStatus,
    #[serde(default)]
    pub pinned: bool,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostUpdateRequest {
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub body_markdown: String,
    pub summary_markdown: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostPinRequest {
    pub pinned: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostTitleSlugRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostTitleRequest {
    pub title: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostExcerptRequest {
    pub excerpt: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostBodyRequest {
    pub body_markdown: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostSummaryRequest {
    pub summary_markdown: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostStatusRequest {
    pub status: PostStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostTagsRequest {
    pub tag_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PageCreateRequest {
    pub slug: Option<String>,
    pub title: String,
    pub body_markdown: String,
    #[serde(default = "default_page_status")]
    pub status: PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PageUpdateRequest {
    pub slug: String,
    pub title: String,
    pub body_markdown: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PageTitleSlugRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PageTitleRequest {
    pub title: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PageBodyRequest {
    pub body_markdown: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PageStatusRequest {
    pub status: PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TagCreateRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TagUpdateRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TagPinRequest {
    pub pinned: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TagNameRequest {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TagDescriptionRequest {
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NavigationCreateRequest {
    pub label: String,
    pub destination_type: NavigationDestinationType,
    pub destination_page_id: Option<Uuid>,
    pub destination_url: Option<String>,
    pub sort_order: i32,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub open_in_new_tab: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NavigationUpdateRequest {
    pub label: String,
    pub destination_type: NavigationDestinationType,
    pub destination_page_id: Option<Uuid>,
    pub destination_url: Option<String>,
    pub sort_order: i32,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub open_in_new_tab: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NavigationLabelRequest {
    pub label: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NavigationDestinationRequest {
    pub destination_type: NavigationDestinationType,
    pub destination_page_id: Option<Uuid>,
    pub destination_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NavigationSortOrderRequest {
    pub sort_order: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NavigationVisibilityRequest {
    pub visible: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NavigationOpenInNewTabRequest {
    pub open_in_new_tab: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SettingsPatchRequest {
    pub brand_title: Option<String>,
    pub brand_href: Option<String>,
    pub footer_copy: Option<String>,
    pub homepage_size: Option<i32>,
    pub admin_page_size: Option<i32>,
    pub show_tag_aggregations: Option<bool>,
    pub show_month_aggregations: Option<bool>,
    pub tag_filter_limit: Option<i32>,
    pub month_filter_limit: Option<i32>,
    pub timezone: Option<String>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub public_site_url: Option<String>,
    pub global_toc_enabled: Option<bool>,
    pub favicon_svg: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum: String,
    pub stored_path: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotListQuery {
    pub entity_type: Option<SnapshotEntityType>,
    pub entity_id: Option<Uuid>,
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotCreateRequest {
    pub entity_type: SnapshotEntityType,
    pub entity_id: Uuid,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SnapshotResponse {
    pub id: Uuid,
    pub entity_type: SnapshotEntityType,
    pub entity_id: Uuid,
    pub version: i32,
    pub description: Option<String>,
    pub schema_version: i64,
    pub content: serde_json::Value,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyInfoResponse {
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<ApiScope>,
    pub status: ApiKeyStatus,
    pub expires_at: Option<OffsetDateTime>,
    pub revoked_at: Option<OffsetDateTime>,
    pub last_used_at: Option<OffsetDateTime>,
}
