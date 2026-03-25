use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::domain::{
    api_keys::{ApiKeyStatus, ApiScope},
    types::{NavigationDestinationType, PageStatus, PostStatus},
};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct SiteArchive {
    pub(super) migrations: MigrationSnapshot,
    pub(super) site_settings: SiteSettingsSnapshot,
    pub(super) posts: Vec<PostSnapshot>,
    pub(super) pages: Vec<PageSnapshot>,
    pub(super) tags: Vec<TagSnapshot>,
    #[serde(rename = "post_tags")]
    pub(super) post_tags: Vec<PostTagLink>,
    pub(super) navigation_items: Vec<NavigationSnapshot>,
    #[serde(default)]
    pub(super) api_keys: Vec<ApiKeySnapshot>,
}

impl SiteArchive {
    pub(super) fn normalize(&mut self) {
        self.posts.sort_by(|a, b| a.slug.cmp(&b.slug));
        self.pages.sort_by(|a, b| a.slug.cmp(&b.slug));
        self.tags.sort_by(|a, b| a.slug.cmp(&b.slug));
        self.post_tags.sort_by(|a, b| {
            a.post_slug
                .cmp(&b.post_slug)
                .then(a.tag_slug.cmp(&b.tag_slug))
        });
        self.navigation_items
            .sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.label.cmp(&b.label)));
        self.api_keys.sort_by(|a, b| a.prefix.cmp(&b.prefix));
        self.migrations.entries.sort_by_key(|entry| entry.version);
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct MigrationSnapshot {
    pub(super) entries: Vec<MigrationEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct MigrationEntry {
    pub(super) version: i64,
    pub(super) checksum: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct SiteSettingsSnapshot {
    pub(super) homepage_size: i32,
    pub(super) admin_page_size: i32,
    pub(super) show_tag_aggregations: bool,
    pub(super) show_month_aggregations: bool,
    pub(super) tag_filter_limit: i32,
    pub(super) month_filter_limit: i32,
    pub(super) global_toc_enabled: bool,
    pub(super) brand_title: String,
    pub(super) brand_href: String,
    pub(super) footer_copy: String,
    pub(super) public_site_url: String,
    pub(super) favicon_svg: String,
    pub(super) timezone: String,
    pub(super) meta_title: String,
    pub(super) meta_description: String,
    pub(super) og_title: String,
    pub(super) og_description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct PostSnapshot {
    pub(super) slug: String,
    pub(super) title: String,
    pub(super) excerpt: String,
    pub(super) body_markdown: String,
    pub(super) summary_markdown: Option<String>,
    pub(super) status: PostStatus,
    pub(super) pinned: bool,
    pub(super) scheduled_at: Option<OffsetDateTime>,
    pub(super) published_at: Option<OffsetDateTime>,
    pub(super) archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct PageSnapshot {
    pub(super) slug: String,
    pub(super) title: String,
    pub(super) body_markdown: String,
    pub(super) status: PageStatus,
    pub(super) scheduled_at: Option<OffsetDateTime>,
    pub(super) published_at: Option<OffsetDateTime>,
    pub(super) archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct TagSnapshot {
    pub(super) slug: String,
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) pinned: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct PostTagLink {
    pub(super) post_slug: String,
    pub(super) tag_slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct NavigationSnapshot {
    pub(super) label: String,
    pub(super) destination_type: NavigationDestinationType,
    pub(super) destination_url: Option<String>,
    pub(super) destination_page_slug: Option<String>,
    pub(super) sort_order: i32,
    pub(super) open_in_new_tab: bool,
    pub(super) visible: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ApiKeySnapshot {
    pub(super) id: Uuid,
    pub(super) name: String,
    #[serde(default)]
    pub(super) description: Option<String>,
    pub(super) prefix: String,
    pub(super) hashed_secret: Vec<u8>,
    pub(super) scopes: Vec<ApiScope>,
    pub(super) status: ApiKeyStatus,
    #[serde(default)]
    pub(super) expires_in: Option<Duration>,
    #[serde(default)]
    pub(super) expires_at: Option<OffsetDateTime>,
    #[serde(default)]
    pub(super) revoked_at: Option<OffsetDateTime>,
    #[serde(default)]
    pub(super) last_used_at: Option<OffsetDateTime>,
    pub(super) created_by: String,
    pub(super) created_at: OffsetDateTime,
    pub(super) updated_at: OffsetDateTime,
}
