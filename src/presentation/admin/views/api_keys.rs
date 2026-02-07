use askama::Template;

use super::{AdminHiddenField, AdminLayout, AdminPostMonthOption, AdminPostTagOption};

#[derive(Clone)]
pub struct AdminApiScopeOption {
    pub value: String,
    pub label: String,
    pub is_selected: bool,
}

#[derive(Clone)]
pub struct AdminApiScopeDisplay {
    pub slug: String,
    pub label: String,
}

#[derive(Clone)]
pub struct AdminApiKeyRowView {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<AdminApiScopeDisplay>,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub status: String,
    pub status_label: String,
    pub description: Option<String>,
    pub edit_href: String,
    pub revoke_action: String,
    pub rotate_action: String,
    pub delete_action: String,
}

#[derive(Clone)]
pub struct AdminApiKeyStatusFilterView {
    pub status_key: Option<String>,
    pub label: String,
    pub count: u64,
    pub is_active: bool,
}

#[derive(Clone)]
pub struct AdminApiKeyPaginationState {
    pub cursor: Option<String>,
    pub trail: Option<String>,
}

#[derive(Clone)]
pub struct AdminApiKeyListView {
    pub heading: String,
    pub keys: Vec<AdminApiKeyRowView>,
    pub create_action: String,
    pub new_key_href: String,
    pub panel_action: String,
    pub filters: Vec<AdminApiKeyStatusFilterView>,
    pub active_status_key: Option<String>,
    pub filter_search: Option<String>,
    pub filter_scope: Option<String>,
    pub filter_tag: Option<String>,
    pub filter_month: Option<String>,
    pub tag_filter_enabled: bool,
    pub month_filter_enabled: bool,
    pub tag_filter_label: String,
    pub tag_filter_all_label: String,
    pub tag_filter_field: String,
    pub tag_options: Vec<AdminPostTagOption>,
    pub month_options: Vec<AdminPostMonthOption>,
    pub cursor_param: Option<String>,
    pub trail: Option<String>,
    pub previous_page_state: Option<AdminApiKeyPaginationState>,
    pub next_page_state: Option<AdminApiKeyPaginationState>,
    pub available_scopes: Vec<AdminApiScopeOption>,
    /// Generic hidden fields for filter state retention
    pub custom_hidden_fields: Vec<AdminHiddenField>,
}

#[derive(Template)]
#[template(path = "admin/api_keys.html")]
pub struct AdminApiKeysTemplate {
    pub view: AdminLayout<AdminApiKeyListView>,
}

#[derive(Template)]
#[template(path = "admin/api_keys_panel.html")]
pub struct AdminApiKeysPanelTemplate {
    pub content: AdminApiKeyListView,
}

#[derive(Clone)]
pub struct AdminApiKeyExpiresInOption {
    pub value: String,
    pub label: String,
    pub selected: bool,
}

#[derive(Clone)]
pub struct AdminApiKeyScopePickerView {
    pub toggle_action: String,
    pub selected: Vec<AdminApiScopeOption>,
    pub available: Vec<AdminApiScopeOption>,
    pub selected_values: Vec<String>,
}

#[derive(Clone)]
pub struct AdminApiKeyCreatedView {
    pub heading: String,
    pub message: String,
    pub token: String,
    pub copy_toast_action: String,
}

#[derive(Template)]
#[template(path = "admin/api_key_created_panel.html")]
pub struct AdminApiKeyCreatedPanelTemplate {
    pub content: AdminApiKeyCreatedView,
}

#[derive(Clone)]
pub struct AdminApiKeyEditorView {
    pub heading: String,
    pub form_action: String,
    pub name: String,
    pub description: Option<String>,
    pub scope_picker: AdminApiKeyScopePickerView,
    pub expires_in_options: Option<Vec<AdminApiKeyExpiresInOption>>,
    pub submit_label: String,
    pub show_back_link: bool,
}

#[derive(Template)]
#[template(path = "admin/api_key_new.html")]
pub struct AdminApiKeyNewTemplate {
    pub view: AdminLayout<AdminApiKeyEditorView>,
}

#[derive(Template)]
#[template(path = "admin/api_key_edit.html")]
pub struct AdminApiKeyEditTemplate {
    pub view: AdminLayout<AdminApiKeyEditorView>,
}

#[derive(Template)]
#[template(path = "admin/api_key_editor_panel.html")]
pub struct AdminApiKeyEditorPanelTemplate {
    pub content: AdminApiKeyEditorView,
}

#[derive(Template)]
#[template(path = "admin/api_key_scope_picker.html")]
pub struct AdminApiKeyScopePickerTemplate {
    pub picker: AdminApiKeyScopePickerView,
}

#[derive(Template)]
#[template(path = "admin/api_key_scope_selection_store.html")]
pub struct AdminApiKeyScopeSelectionStoreTemplate {
    pub picker: AdminApiKeyScopePickerView,
}
