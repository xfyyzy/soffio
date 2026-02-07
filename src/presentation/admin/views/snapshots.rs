use askama::Template;

use super::{
    AdminHiddenField, AdminLayout, AdminPostMonthOption, AdminPostPaginationState,
    AdminPostTagOption,
};

#[derive(Clone)]
pub struct AdminSnapshotRowView {
    pub id: String,
    pub version: i32,
    pub description: Option<String>,
    pub created_at: String,
    pub preview_href: String,
    pub edit_href: String,
    pub rollback_action: String,
    pub delete_action: String,
}

#[derive(Clone)]
pub struct AdminSnapshotListView {
    pub heading: String,
    pub entity_label: String,
    pub snapshots: Vec<AdminSnapshotRowView>,
    pub filter_search: Option<String>,
    pub filter_tag: Option<String>,
    pub filter_month: Option<String>,
    pub month_options: Vec<AdminPostMonthOption>,
    pub tag_options: Vec<AdminPostTagOption>,
    pub tag_filter_label: String,
    pub tag_filter_all_label: String,
    pub tag_filter_field: String,
    pub tag_filter_enabled: bool,
    pub month_filter_enabled: bool,
    pub new_snapshot_href: String,
    pub panel_action: String,
    pub next_cursor: Option<String>,
    pub cursor_param: Option<String>,
    pub trail: Option<String>,
    pub previous_page_state: Option<AdminPostPaginationState>,
    pub next_page_state: Option<AdminPostPaginationState>,
    pub custom_hidden_fields: Vec<AdminHiddenField>,
    pub active_status_key: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/snapshots.html")]
pub struct AdminSnapshotsTemplate {
    pub view: AdminLayout<AdminSnapshotListView>,
}

#[derive(Template)]
#[template(path = "admin/snapshots_panel.html")]
pub struct AdminSnapshotsPanelTemplate {
    pub content: AdminSnapshotListView,
}

#[derive(Clone)]
pub struct AdminSnapshotEditorView {
    pub heading: String,
    pub entity_label: String,
    pub form_action: String,
    pub back_href: String,
    pub version: i32,
    pub description: Option<String>,
    pub submit_label: String,
}

#[derive(Template)]
#[template(path = "admin/snapshot_new.html")]
pub struct AdminSnapshotNewTemplate {
    pub view: AdminLayout<AdminSnapshotEditorView>,
}

#[derive(Template)]
#[template(path = "admin/snapshot_edit.html")]
pub struct AdminSnapshotEditTemplate {
    pub view: AdminLayout<AdminSnapshotEditorView>,
}

#[derive(Template)]
#[template(path = "admin/snapshot_editor_panel.html")]
pub struct AdminSnapshotEditorPanelTemplate {
    pub content: AdminSnapshotEditorView,
}
