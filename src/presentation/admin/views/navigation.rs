use askama::Template;

use super::{
    AdminHiddenField, AdminLayout, AdminPostMonthOption, AdminPostPaginationState,
    AdminPostTagOption,
};

#[derive(Clone)]
pub struct AdminNavigationStatusFilterView {
    pub status_key: Option<String>,
    pub label: String,
    pub count: u64,
    pub is_active: bool,
}

#[derive(Clone)]
pub struct AdminNavigationRowView {
    pub id: String,
    pub label: String,
    pub preview_href: String,
    pub destination_type_label: String,
    pub destination_type_status: String,
    pub destination_display: String,
    pub sort_order: i32,
    pub visible: bool,
    pub toggle_action: String,
    pub toggle_label: &'static str,
    pub edit_href: String,
    pub delete_action: String,
}

#[derive(Clone)]
pub struct AdminNavigationListView {
    pub heading: String,
    pub filters: Vec<AdminNavigationStatusFilterView>,
    pub items: Vec<AdminNavigationRowView>,
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
    pub tag_filter_enabled: bool,
    pub month_filter_enabled: bool,
    pub panel_action: String,
    pub active_status_key: Option<String>,
    pub new_navigation_href: String,
    pub tag_filter_label: String,
    pub tag_filter_all_label: String,
    pub tag_filter_field: String,
    /// Generic hidden fields for filter state retention
    pub custom_hidden_fields: Vec<AdminHiddenField>,
}

#[derive(Template)]
#[template(path = "admin/navigation.html")]
pub struct AdminNavigationTemplate {
    pub view: AdminLayout<AdminNavigationListView>,
}

#[derive(Template)]
#[template(path = "admin/navigation_panel.html")]
pub struct AdminNavigationPanelTemplate {
    pub content: AdminNavigationListView,
}

#[derive(Clone)]
pub struct AdminNavigationDestinationTypeOption {
    pub value: &'static str,
    pub label: &'static str,
    pub selected: bool,
}

#[derive(Clone)]
pub struct AdminNavigationPageOption {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub selected: bool,
}

#[derive(Clone)]
pub struct AdminNavigationEditorView {
    pub heading: String,
    pub id: Option<String>,
    pub label: String,
    pub destination_type_options: Vec<AdminNavigationDestinationTypeOption>,
    pub page_options: Vec<AdminNavigationPageOption>,
    pub destination_url: Option<String>,
    pub sort_order: i32,
    pub visible: bool,
    pub open_in_new_tab: bool,
    pub page_has_selection: bool,
    pub form_action: String,
    pub submit_label: String,
    pub enable_live_submit: bool,
    pub active_destination_type: String,
    pub preview_action: String,
    pub visible_input_id: String,
    pub open_in_new_tab_input_id: String,
}

#[derive(Template)]
#[template(path = "admin/navigation_edit.html")]
pub struct AdminNavigationEditTemplate {
    pub view: AdminLayout<AdminNavigationEditorView>,
}

#[derive(Template)]
#[template(path = "admin/navigation_editor_panel.html")]
pub struct AdminNavigationEditPanelTemplate {
    pub content: AdminNavigationEditorView,
}
