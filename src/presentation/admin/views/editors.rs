use crate::domain::types::{PageStatus, PostStatus};
use askama::Template;

use super::AdminLayout;

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
