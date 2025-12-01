//! Domain entities mirrored from persistent storage.

use chrono_tz::Tz;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::{
    types::{JobState, JobType, NavigationDestinationType, PageStatus, PostStatus},
    uploads::UploadMetadata,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PostRecord {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub body_markdown: String,
    pub status: PostStatus,
    pub pinned: bool,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
    pub summary_markdown: Option<String>,
    pub summary_html: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PostSectionRecord {
    pub id: Uuid,
    pub post_id: Uuid,
    pub position: i32,
    pub level: i16,
    pub parent_id: Option<Uuid>,
    pub heading_html: String,
    pub heading_text: String,
    pub body_html: String,
    pub contains_code: bool,
    pub contains_math: bool,
    pub contains_mermaid: bool,
    pub anchor_slug: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PageRecord {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub body_markdown: String,
    pub rendered_html: String,
    pub status: PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TagRecord {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub pinned: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NavigationItemRecord {
    pub id: Uuid,
    pub label: String,
    pub destination_type: NavigationDestinationType,
    pub destination_page_id: Option<Uuid>,
    pub destination_page_slug: Option<String>,
    pub destination_url: Option<String>,
    pub sort_order: i32,
    pub visible: bool,
    pub open_in_new_tab: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SiteSettingsRecord {
    pub homepage_size: i32,
    pub admin_page_size: i32,
    pub show_tag_aggregations: bool,
    pub show_month_aggregations: bool,
    pub tag_filter_limit: i32,
    pub month_filter_limit: i32,
    pub global_toc_enabled: bool,
    pub brand_title: String,
    pub brand_href: String,
    pub footer_copy: String,
    pub public_site_url: String,
    pub favicon_svg: String,
    pub timezone: Tz,
    pub meta_title: String,
    pub meta_description: String,
    pub og_title: String,
    pub og_description: String,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UploadRecord {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub checksum: String,
    pub stored_path: String,
    pub metadata: UploadMetadata,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuditLogRecord {
    pub id: Uuid,
    pub actor: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub payload_text: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobRecord {
    pub id: String,
    pub job_type: JobType,
    pub payload: serde_json::Value,
    pub state: JobState,
    pub attempts: i32,
    pub max_attempts: i32,
    pub run_at: OffsetDateTime,
    pub lock_at: Option<OffsetDateTime>,
    pub lock_by: Option<String>,
    pub done_at: Option<OffsetDateTime>,
    pub last_error: Option<String>,
    pub priority: i32,
}
