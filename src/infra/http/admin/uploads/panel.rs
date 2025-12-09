//! Panel building and view construction for uploads.

use askama::Template;
use std::convert::TryFrom;
use url::form_urlencoded;

use crate::{
    application::{
        admin::uploads::AdminUploadError,
        error::HttpError,
        pagination::UploadCursor,
        repos::{SettingsRepo, UploadQueryFilter},
    },
    domain::{entities::UploadRecord, uploads},
    presentation::admin::views as admin_views,
    util::bytes::format_bytes,
};

use super::super::{
    AdminState,
    pagination::{self, CursorState},
    shared::{blank_to_none_opt, template_render_http_error},
};

pub(super) fn wrap_content(inner_html: String) -> String {
    format!("<div data-admin-content>{inner_html}</div>")
}

pub(super) fn build_upload_filter(
    search: Option<&str>,
    content_type: Option<&str>,
    month: Option<&str>,
) -> UploadQueryFilter {
    UploadQueryFilter {
        search: blank_to_none_opt(search.map(str::to_string)),
        content_type: blank_to_none_opt(content_type.map(str::to_string)),
        month: blank_to_none_opt(month.map(str::to_string)),
    }
}

pub(super) async fn build_upload_list_view(
    state: &AdminState,
    filter: &UploadQueryFilter,
    cursor: Option<UploadCursor>,
) -> Result<admin_views::AdminUploadListView, AdminUploadError> {
    let settings = state.db.load_site_settings().await?;
    let admin_page_size = settings.admin_page_size.clamp(1, 100) as u32;
    let timezone = settings.timezone;
    let public_site_url = normalize_public_site_url(&settings.public_site_url);

    let page_request = crate::application::pagination::PageRequest::new(admin_page_size, cursor);

    let list_filter = filter.clone();
    let mut type_filter = filter.clone();
    type_filter.content_type = None;
    let mut month_filter = filter.clone();
    month_filter.month = None;

    let (page, type_counts, month_counts) = tokio::try_join!(
        state.uploads.list(&list_filter, page_request),
        state.uploads.content_type_counts(&type_filter),
        state.uploads.month_counts(&month_filter)
    )?;

    let uploads = page
        .items
        .into_iter()
        .map(|record| map_upload_row(&record, timezone, &public_site_url))
        .collect();

    let tag_options = type_counts
        .into_iter()
        .map(|entry| admin_views::AdminPostTagOption {
            slug: entry.content_type.clone(),
            name: entry.content_type,
            count: entry.count,
        })
        .collect();

    let month_options = month_counts
        .into_iter()
        .map(|month| admin_views::AdminPostMonthOption {
            key: month.key,
            label: month.label,
            count: month.count as usize,
        })
        .collect();

    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    if let Some(search) = filter.search.as_ref() {
        serializer.append_pair("search", search);
    }
    if let Some(content_type) = filter.content_type.as_ref() {
        serializer.append_pair("content_type", content_type);
    }
    if let Some(month) = filter.month.as_ref() {
        serializer.append_pair("month", month);
    }
    let filter_query = serializer.finish();

    Ok(admin_views::AdminUploadListView {
        heading: "Uploads".to_string(),
        uploads,
        filter_search: filter.search.clone(),
        filter_tag: filter.content_type.clone(),
        filter_month: filter.month.clone(),
        filter_query,
        active_status_key: None,
        tag_options,
        month_options,
        next_cursor: page.next_cursor,
        cursor_param: None,
        trail: None,
        previous_page_state: None,
        next_page_state: None,
        panel_action: "/uploads/panel".to_string(),
        new_upload_href: "/uploads/new".to_string(),
        tag_filter_label: "Content Type".to_string(),
        tag_filter_all_label: "All types".to_string(),
        tag_filter_field: "content_type".to_string(),
        tag_filter_enabled: true,
        month_filter_enabled: true,
        copy_toast_action: "/toasts".to_string(),
        job_type_filter_enabled: false,
        filter_job_type: None,
    })
}

pub(super) fn render_upload_panel_html(
    content: &admin_views::AdminUploadListView,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let template = admin_views::AdminUploadsPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}

pub(super) fn render_upload_form_panel_html(
    content: &admin_views::AdminUploadFormView,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let template = admin_views::AdminUploadNewPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}

pub(super) fn render_upload_queue_html(
    queue: &admin_views::AdminUploadQueueView,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let template = admin_views::AdminUploadQueueTemplate {
        queue: queue.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}

pub(super) fn map_upload_row(
    record: &UploadRecord,
    timezone: chrono_tz::Tz,
    public_site_url: &str,
) -> admin_views::AdminUploadRowView {
    let created_at = admin_views::format_timestamp(record.created_at, timezone);
    let size_label = match u64::try_from(record.size_bytes) {
        Ok(value) => format_bytes(value),
        Err(_) => format_bytes(0),
    };
    let mut public_href = format!("{}uploads/{}", public_site_url, record.stored_path);
    let mut query = form_urlencoded::Serializer::new(String::new());
    for (key, value) in record.metadata.query_pairs() {
        query.append_pair(&key, &value);
    }
    let query = query.finish();
    if !query.is_empty() {
        public_href.push('?');
        public_href.push_str(&query);
    }
    let preview_href = if uploads::supports_inline_preview(&record.content_type) {
        Some(public_href.clone())
    } else {
        None
    };

    admin_views::AdminUploadRowView {
        id: record.id.to_string(),
        filename: record.filename.clone(),
        content_type: record.content_type.clone(),
        size_bytes: record.size_bytes,
        size_label,
        created_at,
        download_href: format!("/uploads/{}", record.id),
        delete_action: format!("/uploads/{}/delete", record.id),
        preview_href,
        public_href,
    }
}

pub(super) fn apply_upload_pagination_links(
    content: &mut admin_views::AdminUploadListView,
    cursor_state: &CursorState,
) {
    content.cursor_param = cursor_state.current_token();
    content.trail = pagination::join_cursor_history(cursor_state.history_tokens());

    let mut previous_history = cursor_state.clone_history();
    let previous_token = previous_history.pop();

    content.previous_page_state = previous_token.map(|token| {
        let previous_cursor_value = pagination::decode_cursor_token(&token);
        let previous_trail = pagination::join_cursor_history(&previous_history);
        admin_views::AdminPostPaginationState {
            cursor: previous_cursor_value,
            trail: previous_trail,
        }
    });

    if let Some(next_cursor) = content.next_cursor.clone() {
        let mut next_history = cursor_state.clone_history();
        next_history.push(pagination::encode_cursor_token(
            cursor_state.current_token_ref(),
        ));
        let next_trail = pagination::join_cursor_history(&next_history);
        content.next_page_state = Some(admin_views::AdminPostPaginationState {
            cursor: Some(next_cursor),
            trail: next_trail,
        });
    } else {
        content.next_page_state = None;
    }
}

pub(super) fn normalize_public_site_url(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    }
}
