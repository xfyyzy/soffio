use askama::Template;

use super::{AdminHiddenField, AdminLayout};

#[derive(Clone)]
pub struct AdminPostRowView {
    pub id: String,
    pub title: String,
    pub status_key: String,
    pub status_label: String,
    pub display_time: Option<String>,
    pub display_time_kind: AdminPostTimeKind,
    pub actions: Vec<AdminPostRowActionView>,
    pub preview_href: String,
    pub edit_href: String,
    pub is_pinned: bool,
    pub snapshots_href: Option<String>,
}

#[derive(Clone, Copy)]
pub enum AdminPostTimeKind {
    Published,
    Updated,
}

#[derive(Clone)]
pub struct AdminPostRowActionView {
    pub value: &'static str,
    pub label: &'static str,
    pub is_danger: bool,
}

#[derive(Clone)]
pub struct AdminPostTagOption {
    pub slug: String,
    pub name: String,
    pub count: u64,
}

#[derive(Clone)]
pub struct AdminPostMonthOption {
    pub key: String,
    pub label: String,
    pub count: usize,
}

#[derive(Clone)]
pub struct AdminPostStatusFilterView {
    pub status_key: Option<String>,
    pub label: String,
    pub count: u64,
    pub is_active: bool,
}

#[derive(Clone)]
pub struct AdminPostPaginationState {
    pub cursor: Option<String>,
    pub trail: Option<String>,
}

#[derive(Clone)]
pub struct AdminPostListView {
    pub heading: String,
    pub filters: Vec<AdminPostStatusFilterView>,
    pub posts: Vec<AdminPostRowView>,
    pub tag_options: Vec<AdminPostTagOption>,
    pub month_options: Vec<AdminPostMonthOption>,
    pub filter_search: Option<String>,
    pub filter_tag: Option<String>,
    pub filter_month: Option<String>,
    pub next_cursor: Option<String>,
    pub filter_query: String,
    pub cursor_param: Option<String>,
    pub trail: Option<String>,
    pub previous_page_state: Option<AdminPostPaginationState>,
    pub next_page_state: Option<AdminPostPaginationState>,
    pub time_column_label: String,
    pub new_post_href: String,
    pub public_site_url: String,
    pub active_status_key: Option<String>,
    pub panel_action: String,
    pub tag_filter_label: String,
    pub tag_filter_all_label: String,
    pub tag_filter_field: String,
    pub tag_filter_enabled: bool,
    pub month_filter_enabled: bool,
    pub row_action_prefix: String,
    /// Generic hidden fields for filter state retention (replaces hardcoded fields)
    pub custom_hidden_fields: Vec<AdminHiddenField>,
}

#[derive(Template)]
#[template(path = "admin/posts.html")]
pub struct AdminPostsTemplate {
    pub view: AdminLayout<AdminPostListView>,
}

#[derive(Template)]
#[template(path = "admin/posts_panel.html")]
pub struct AdminPostsPanelTemplate {
    pub content: AdminPostListView,
}
