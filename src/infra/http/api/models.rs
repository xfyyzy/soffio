use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::types::{NavigationDestinationType, PageStatus, PostStatus};

fn default_post_status() -> PostStatus {
    PostStatus::Draft
}

fn default_page_status() -> PageStatus {
    PageStatus::Draft
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct PostUpdateRequest {
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub body_markdown: String,
    pub summary_markdown: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Deserialize)]
pub struct PostStatusRequest {
    pub status: PostStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct PostTagsRequest {
    pub tag_ids: Vec<Uuid>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct PageUpdateRequest {
    pub slug: String,
    pub title: String,
    pub body_markdown: String,
}

#[derive(Debug, Deserialize)]
pub struct PageStatusRequest {
    pub status: PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct TagCreateRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Deserialize)]
pub struct TagUpdateRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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
