use crate::domain::{entities::PageRecord, types::PageStatus};
use crate::presentation::admin::views as admin_views;
use chrono_tz::Tz;

use super::status::page_status_options;

pub(crate) fn build_page_editor_view(
    page: &PageRecord,
    tz: Tz,
) -> admin_views::AdminPageEditorView {
    admin_views::AdminPageEditorView {
        title: page.title.clone(),
        heading: format!("Edit Page: {}", page.title),
        body_markdown: page.body_markdown.clone(),
        status: page.status,
        status_options: page_status_options(page.status),
        published_at: page
            .published_at
            .map(|time| admin_views::format_timestamp(time, tz)),
        form_action: format!("/pages/{}/edit", page.id),
        submit_label: "Save Changes".to_string(),
        enable_live_submit: true,
    }
}

pub(crate) fn build_new_page_editor_view() -> admin_views::AdminPageEditorView {
    admin_views::AdminPageEditorView {
        title: String::new(),
        heading: "Create Page".to_string(),
        body_markdown: String::new(),
        status: PageStatus::Draft,
        status_options: page_status_options(PageStatus::Draft),
        published_at: None,
        form_action: "/pages/create".to_string(),
        submit_label: "Create Page".to_string(),
        enable_live_submit: true,
    }
}
