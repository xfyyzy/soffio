use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::api_keys::{ApiKeyStatus, ApiScope};
use crate::domain::types::{NavigationDestinationType, PageStatus, PostStatus};

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

#[allow(dead_code)]
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
