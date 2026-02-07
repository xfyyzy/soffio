use askama::Template;
use serde_json::json;

use super::{
    AdminHiddenField, AdminLayout, AdminPostMonthOption, AdminPostPaginationState,
    AdminPostTagOption,
};

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
    /// Generic hidden fields for filter state retention
    pub custom_hidden_fields: Vec<AdminHiddenField>,
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
