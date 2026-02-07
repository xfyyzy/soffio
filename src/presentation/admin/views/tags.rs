use askama::Template;

use super::{
    AdminHiddenField, AdminLayout, AdminPageStatusFilterView, AdminPostMonthOption,
    AdminPostPaginationState, AdminPostTagOption, AdminPostTimeKind,
};

#[derive(Clone)]
pub struct AdminTagRowView {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub usage_count: u64,
    pub pinned: bool,
    pub display_time: Option<String>,
    pub display_time_kind: AdminPostTimeKind,
    pub public_href: String,
    pub edit_href: String,
    pub pin_action: String,
    pub unpin_action: String,
    pub delete_action: String,
}

#[derive(Clone)]
pub struct AdminTagListView {
    pub heading: String,
    pub tags: Vec<AdminTagRowView>,
    pub filter_search: Option<String>,
    pub filter_month: Option<String>,
    pub filter_tag: Option<String>,
    pub filter_query: String,
    pub tag_options: Vec<AdminPostTagOption>,
    pub filters: Vec<AdminPageStatusFilterView>,
    pub month_options: Vec<AdminPostMonthOption>,
    pub next_cursor: Option<String>,
    pub cursor_param: Option<String>,
    pub trail: Option<String>,
    pub previous_page_state: Option<AdminPostPaginationState>,
    pub next_page_state: Option<AdminPostPaginationState>,
    pub active_status_key: Option<String>,
    pub panel_action: String,
    pub new_tag_href: String,
    pub time_column_label: String,
    pub month_filter_enabled: bool,
    pub tag_filter_enabled: bool,
    pub tag_filter_label: String,
    pub tag_filter_all_label: String,
    pub tag_filter_field: String,
    /// Generic hidden fields for filter state retention
    pub custom_hidden_fields: Vec<AdminHiddenField>,
}

#[derive(Template)]
#[template(path = "admin/tags.html")]
pub struct AdminTagsTemplate {
    pub view: AdminLayout<AdminTagListView>,
}

#[derive(Template)]
#[template(path = "admin/tags_panel.html")]
pub struct AdminTagsPanelTemplate {
    pub content: AdminTagListView,
}

#[derive(Clone)]
pub struct AdminTagEditView {
    pub heading: String,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub pinned: bool,
    pub form_action: String,
    pub submit_label: String,
    pub pin_label: String,
}

#[derive(Template)]
#[template(path = "admin/tag_edit.html")]
pub struct AdminTagEditTemplate {
    pub view: AdminLayout<AdminTagEditView>,
}

#[derive(Template)]
#[template(path = "admin/tag_editor_panel.html")]
pub struct AdminTagEditPanelTemplate {
    pub content: AdminTagEditView,
}
