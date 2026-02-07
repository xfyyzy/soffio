use askama::Template;

use super::{
    AdminHiddenField, AdminLayout, AdminPostMonthOption, AdminPostPaginationState,
    AdminPostRowActionView, AdminPostTagOption, AdminPostTimeKind,
};

#[derive(Clone)]
pub struct AdminPageRowView {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub status_key: String,
    pub status_label: String,
    pub display_time: Option<String>,
    pub display_time_kind: AdminPostTimeKind,
    pub actions: Vec<AdminPostRowActionView>,
    pub preview_href: String,
    pub edit_href: String,
    pub snapshots_href: Option<String>,
}

#[derive(Clone)]
pub struct AdminPageStatusFilterView {
    pub status_key: Option<String>,
    pub label: String,
    pub count: u64,
    pub is_active: bool,
}

#[derive(Clone)]
pub struct AdminPageListView {
    pub heading: String,
    pub filters: Vec<AdminPageStatusFilterView>,
    pub pages: Vec<AdminPageRowView>,
    pub filter_search: Option<String>,
    pub filter_tag: Option<String>,
    pub filter_month: Option<String>,
    pub filter_query: String,
    pub next_cursor: Option<String>,
    pub cursor_param: Option<String>,
    pub trail: Option<String>,
    pub previous_page_state: Option<AdminPostPaginationState>,
    pub next_page_state: Option<AdminPostPaginationState>,
    pub tag_options: Vec<AdminPostTagOption>,
    pub month_options: Vec<AdminPostMonthOption>,
    pub time_column_label: String,
    pub new_page_href: String,
    pub public_site_url: String,
    pub active_status_key: Option<String>,
    pub panel_action: String,
    pub tag_filter_label: String,
    pub tag_filter_all_label: String,
    pub tag_filter_field: String,
    pub tag_filter_enabled: bool,
    pub month_filter_enabled: bool,
    pub row_action_prefix: String,
    /// Generic hidden fields for filter state retention
    pub custom_hidden_fields: Vec<AdminHiddenField>,
}

#[derive(Template)]
#[template(path = "admin/pages.html")]
pub struct AdminPagesTemplate {
    pub view: AdminLayout<AdminPageListView>,
}

#[derive(Template)]
#[template(path = "admin/pages_panel.html")]
pub struct AdminPagesPanelTemplate {
    pub content: AdminPageListView,
}
