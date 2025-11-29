use crate::domain::types::{PageStatus, PostStatus};
use crate::util::timezone;
use askama::Template;
use chrono_tz::Tz;
use serde_json::json;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct AdminBrandView {
    pub title: String,
}

#[derive(Clone)]
pub struct AdminNavigationItemView {
    pub label: String,
    pub href: String,
    pub is_active: bool,
    pub open_in_new_tab: bool,
}

#[derive(Clone)]
pub struct AdminNavigationView {
    pub items: Vec<AdminNavigationItemView>,
}

#[derive(Clone)]
pub struct AdminMetaView {
    pub title: String,
    pub description: String,
}

#[derive(Clone)]
pub struct AdminChrome {
    pub brand: AdminBrandView,
    pub navigation: AdminNavigationView,
    pub meta: AdminMetaView,
}

#[derive(Clone)]
pub struct AdminLayout<T> {
    pub chrome: AdminChrome,
    pub content: T,
}

impl<T> AdminLayout<T> {
    pub fn new(chrome: AdminChrome, content: T) -> Self {
        Self { chrome, content }
    }
}

#[derive(Clone)]
pub struct AdminMetricView {
    pub label: String,
    pub value: u64,
    pub hint: Option<String>,
}

#[derive(Clone)]
pub struct AdminDashboardPanelView {
    pub title: String,
    pub caption: String,
    pub metrics: Vec<AdminMetricView>,
    pub empty_message: String,
}

impl AdminDashboardPanelView {
    pub fn has_metrics(&self) -> bool {
        !self.metrics.is_empty()
    }
}

#[derive(Clone)]
pub struct AdminDashboardView {
    pub title: String,
    pub panels: Vec<AdminDashboardPanelView>,
    pub empty_message: String,
}

impl AdminDashboardView {
    pub fn has_panels(&self) -> bool {
        !self.panels.is_empty()
    }
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct AdminDashboardTemplate {
    pub view: AdminLayout<AdminDashboardView>,
}

#[derive(Clone)]
pub struct AdminFlashMessage {
    pub kind: &'static str,
    pub text: String,
}

#[derive(Clone)]
pub struct AdminToastItem {
    pub id: String,
    pub kind: &'static str,
    pub text: String,
    pub ttl_ms: u64,
}

#[derive(Template)]
#[template(path = "admin/toast_stack.html")]
pub struct AdminToastStackTemplate {
    pub toasts: Vec<AdminToastItem>,
}

#[derive(Clone)]
pub struct AdminPostRowView {
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
    pub is_pinned: bool,
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

pub fn format_timestamp(time: OffsetDateTime, tz: Tz) -> String {
    let localized = timezone::localized_datetime(time, tz);
    localized.format("%Y/%m/%d %H:%M:%S").to_string()
}

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

#[derive(Clone)]
pub struct AdminSettingsSummaryView {
    pub heading: String,
    pub simple_fields: Vec<AdminSettingsSummaryField>,
    pub multiline_fields: Vec<AdminSettingsSummaryField>,
    pub updated_at: String,
    pub edit_href: String,
}

#[derive(Clone)]
pub struct AdminSettingsSummaryField {
    pub label: String,
    pub value: String,
    pub value_kind: AdminSettingsSummaryValueKind,
}

#[derive(Clone)]
pub enum AdminSettingsSummaryValueKind {
    Text,
    Multiline,
    Badge {
        status: &'static str,
        label: &'static str,
    },
}

#[derive(Template)]
#[template(path = "admin/settings.html")]
pub struct AdminSettingsTemplate {
    pub view: AdminLayout<AdminSettingsSummaryView>,
}

#[derive(Template)]
#[template(path = "admin/settings_panel.html")]
pub struct AdminSettingsPanelTemplate {
    pub content: AdminSettingsSummaryView,
}

#[derive(Clone)]
pub struct AdminSettingsEditView {
    pub heading: String,
    pub simple_fields: Vec<AdminSettingsEditSimpleField>,
    pub multiline_fields: Vec<AdminSettingsEditMultilineField>,
    pub updated_at: String,
    pub form_action: String,
    pub submit_label: String,
    pub enable_live_submit: bool,
}

#[derive(Clone)]
pub struct AdminSettingsEditSimpleField {
    pub label: String,
    pub input: AdminSettingsEditInputKind,
}

#[derive(Clone)]
pub enum AdminSettingsEditInputKind {
    Number {
        name: String,
        value: String,
        min: Option<String>,
    },
    Text {
        name: String,
        value: String,
        placeholder: Option<String>,
    },
    Checkbox {
        name: String,
        checked: bool,
        toggle_id: String,
    },
}

#[derive(Clone)]
pub struct AdminSettingsEditMultilineField {
    pub label: String,
    pub name: String,
    pub value: String,
    pub rows: u32,
}

#[derive(Template)]
#[template(path = "admin/settings_edit.html")]
pub struct AdminSettingsEditTemplate {
    pub view: AdminLayout<AdminSettingsEditView>,
}

#[derive(Template)]
#[template(path = "admin/settings_editor_panel.html")]
pub struct AdminSettingsEditPanelTemplate {
    pub content: AdminSettingsEditView,
}

#[derive(Clone)]
pub struct AdminUploadRowView {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub size_label: String,
    pub created_at: String,
    pub download_href: String,
    pub delete_action: String,
    pub preview_href: Option<String>,
    pub public_href: String,
}

#[derive(Clone)]
pub struct AdminUploadListView {
    pub heading: String,
    pub uploads: Vec<AdminUploadRowView>,
    pub filter_search: Option<String>,
    pub filter_tag: Option<String>,
    pub filter_month: Option<String>,
    pub filter_query: String,
    pub active_status_key: Option<String>,
    pub tag_options: Vec<AdminPostTagOption>,
    pub month_options: Vec<AdminPostMonthOption>,
    pub next_cursor: Option<String>,
    pub cursor_param: Option<String>,
    pub trail: Option<String>,
    pub previous_page_state: Option<AdminPostPaginationState>,
    pub next_page_state: Option<AdminPostPaginationState>,
    pub panel_action: String,
    pub new_upload_href: String,
    pub tag_filter_label: String,
    pub tag_filter_all_label: String,
    pub tag_filter_field: String,
    pub tag_filter_enabled: bool,
    pub month_filter_enabled: bool,
    pub copy_toast_action: String,
}

#[derive(Clone)]
pub struct AdminUploadQueueEntry {
    pub id: Option<String>,
    pub filename: String,
    pub size_bytes: u64,
    pub size_label: String,
    pub status: String,
    pub status_label: String,
    pub message: Option<String>,
}

impl AdminUploadQueueEntry {
    pub fn status_class(&self) -> &str {
        self.status.as_str()
    }
}

#[derive(Clone)]
pub struct AdminUploadQueueView {
    pub entries: Vec<AdminUploadQueueEntry>,
    pub limit_mib: u64,
}

impl AdminUploadQueueView {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn manifest_json(&self) -> String {
        let manifest: Vec<_> = self
            .entries
            .iter()
            .map(|entry| {
                json!({
                    "id": entry.id.as_deref(),
                    "filename": entry.filename,
                    "size_bytes": entry.size_bytes,
                    "status": entry.status,
                    "message": entry.message,
                })
            })
            .collect();

        serde_json::to_string(&manifest).unwrap_or_else(|_| "[]".to_string())
    }
}

#[derive(Template)]
#[template(path = "admin/uploads.html")]
pub struct AdminUploadsTemplate {
    pub view: AdminLayout<AdminUploadListView>,
}

#[derive(Template)]
#[template(path = "admin/uploads_panel.html")]
pub struct AdminUploadsPanelTemplate {
    pub content: AdminUploadListView,
}

#[derive(Template)]
#[template(path = "admin/upload_queue.html")]
pub struct AdminUploadQueueTemplate {
    pub queue: AdminUploadQueueView,
}

#[derive(Clone)]
pub struct AdminUploadFormView {
    pub heading: String,
    pub upload_action: String,
    pub queue_sync_action: String,
    pub back_href: String,
    pub toast_action: String,
    pub upload_limit_bytes: u64,
    pub upload_limit_mib: u64,
    pub queue: AdminUploadQueueView,
}

#[derive(Template)]
#[template(path = "admin/upload_new.html")]
pub struct AdminUploadNewTemplate {
    pub view: AdminLayout<AdminUploadFormView>,
}

#[derive(Template)]
#[template(path = "admin/upload_new_panel.html")]
pub struct AdminUploadNewPanelTemplate {
    pub content: AdminUploadFormView,
}

#[derive(Clone)]
pub struct AdminJobRowView {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub scheduled_at: Option<String>,
    pub created_at: String,
    pub error_text: Option<String>,
    pub detail_href: String,
    pub retry_action: String,
    pub cancel_action: String,
    pub can_retry: bool,
    pub can_cancel: bool,
}

#[derive(Clone)]
pub struct AdminJobFilterOption {
    pub value: String,
    pub label: String,
    pub selected: bool,
}

#[derive(Clone)]
pub struct AdminJobListView {
    pub heading: String,
    pub jobs: Vec<AdminJobRowView>,
    pub state_options: Vec<AdminJobFilterOption>,
    pub type_options: Vec<AdminJobFilterOption>,
    pub filter_search: Option<String>,
    pub filter_state: Option<String>,
    pub filter_job_type: Option<String>,
    pub filter_query: String,
    pub current_cursor: Option<String>,
    pub next_cursor: Option<String>,
    pub flash: Option<AdminFlashMessage>,
}

#[derive(Template)]
#[template(path = "admin/jobs.html")]
pub struct AdminJobsTemplate {
    pub view: AdminLayout<AdminJobListView>,
}

#[derive(Clone)]
pub struct AdminJobDetailView {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub run_at: String,
    pub lock_at: Option<String>,
    pub lock_by: Option<String>,
    pub done_at: Option<String>,
    pub last_error: Option<String>,
    pub priority: i32,
    pub payload_pretty: String,
    pub retry_action: String,
    pub cancel_action: String,
    pub back_href: String,
    pub filter_state: Option<String>,
    pub filter_job_type: Option<String>,
    pub filter_search: Option<String>,
    pub filter_cursor: Option<String>,
    pub can_retry: bool,
    pub can_cancel: bool,
    pub flash: Option<AdminFlashMessage>,
    pub filter_query: String,
}

#[derive(Template)]
#[template(path = "admin/job_detail.html")]
pub struct AdminJobDetailTemplate {
    pub view: AdminLayout<AdminJobDetailView>,
}

#[derive(Clone)]
pub struct AdminAuditRowView {
    pub id: String,
    pub actor: String,
    pub action: String,
    pub entity: String,
    pub created_at: String,
}

#[derive(Clone)]
pub struct AdminAuditListView {
    pub heading: String,
    pub entries: Vec<AdminAuditRowView>,
    pub filter_actor: Option<String>,
    pub filter_action: Option<String>,
    pub filter_entity_type: Option<String>,
    pub filter_search: Option<String>,
    pub filter_query: String,
    pub next_cursor: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/audit.html")]
pub struct AdminAuditTemplate {
    pub view: AdminLayout<AdminAuditListView>,
}

#[derive(Clone)]
pub struct AdminPostEditorView {
    pub title: String,
    pub heading: String,
    pub excerpt: String,
    pub body_markdown: String,
    pub summary_markdown: Option<String>,
    pub status: PostStatus,
    pub status_options: Vec<AdminPostStatusOption>,
    pub published_at: Option<String>,
    pub form_action: String,
    pub submit_label: String,
    pub enable_live_submit: bool,
    pub tag_picker: AdminPostTagPickerView,
    pub pinned: bool,
}

#[derive(Clone)]
pub struct AdminPostTagPickerView {
    pub toggle_action: String,
    pub options: Vec<AdminPostTagPickerOptionView>,
    pub selected: Vec<AdminPostSelectedTagView>,
    pub selected_tag_ids: Vec<String>,
}

#[derive(Clone)]
pub struct AdminPostTagPickerOptionView {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub usage_count: i64,
    pub is_selected: bool,
}

#[derive(Clone)]
pub struct AdminPostSelectedTagView {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Clone)]
pub struct AdminPageEditorView {
    pub title: String,
    pub heading: String,
    pub body_markdown: String,
    pub status: PageStatus,
    pub status_options: Vec<AdminPageStatusOption>,
    pub published_at: Option<String>,
    pub form_action: String,
    pub submit_label: String,
    pub enable_live_submit: bool,
}

#[derive(Clone)]
pub struct AdminPageStatusOption {
    pub value: &'static str,
    pub label: &'static str,
    pub selected: bool,
}

#[derive(Clone)]
pub struct AdminPostStatusOption {
    pub value: &'static str,
    pub label: &'static str,
    pub selected: bool,
}

#[derive(Template)]
#[template(path = "admin/post_edit.html")]
pub struct AdminPostEditTemplate {
    pub view: AdminLayout<AdminPostEditorView>,
}

#[derive(Template)]
#[template(path = "admin/post_editor_panel.html")]
pub struct AdminPostEditPanelTemplate {
    pub content: AdminPostEditorView,
}

#[derive(Template)]
#[template(path = "admin/post_tag_picker.html")]
pub struct AdminPostTagPickerTemplate {
    pub picker: AdminPostTagPickerView,
}

#[derive(Template)]
#[template(path = "admin/post_tag_selection_store.html")]
pub struct AdminPostTagSelectionStoreTemplate {
    pub picker: AdminPostTagPickerView,
}

#[derive(Clone)]
pub struct AdminApiScopeOption {
    pub value: String,
    pub label: String,
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
    pub new_token: Option<String>,
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
pub struct AdminApiKeyNewView {
    pub heading: String,
    pub form_action: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub expires_in_options: Vec<AdminApiKeyExpiresInOption>,
    pub scope_picker: AdminApiKeyScopePickerView,
    pub new_token: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/api_key_new.html")]
pub struct AdminApiKeyNewTemplate {
    pub view: AdminLayout<AdminApiKeyNewView>,
}

#[derive(Template)]
#[template(path = "admin/api_key_new_panel.html")]
pub struct AdminApiKeyNewPanelTemplate {
    pub content: AdminApiKeyNewView,
}

#[derive(Clone)]
pub struct AdminApiKeyCreatedView {
    pub heading: String,
    pub token: String,
    pub back_href: String,
    pub copy_toast_action: String,
}

#[derive(Template)]
#[template(path = "admin/api_key_created_panel.html")]
pub struct AdminApiKeyCreatedPanelTemplate {
    pub content: AdminApiKeyCreatedView,
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

#[derive(Template)]
#[template(path = "admin/page_edit.html")]
pub struct AdminPageEditTemplate {
    pub view: AdminLayout<AdminPageEditorView>,
}

#[derive(Template)]
#[template(path = "admin/page_editor_panel.html")]
pub struct AdminPageEditPanelTemplate {
    pub content: AdminPageEditorView,
}
