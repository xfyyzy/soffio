//! Response helper functions for upload handlers.

use axum::{
    body::Body,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use bytes::Bytes;

use crate::{
    application::{error::HttpError, pagination::UploadCursor, repos::UploadQueryFilter},
    domain::entities::UploadRecord,
    presentation::admin::views as admin_views,
};

use super::super::{
    AdminState,
    pagination::CursorState,
    selectors::{ADMIN_CONTENT, UPLOADS_PANEL},
    shared::{Toast, datastar_replace, push_toasts},
};

use super::errors::admin_upload_error;
use super::panel::{
    apply_upload_pagination_links, build_upload_list_view, render_upload_form_panel_html,
    render_upload_panel_html, wrap_content,
};

pub(super) async fn respond_with_upload_form(state: &AdminState, toast: Toast) -> Response {
    let content = admin_views::AdminUploadFormView {
        heading: "Upload file".to_string(),
        upload_action: "/uploads".to_string(),
        queue_sync_action: "/uploads/queue/preview".to_string(),
        back_href: "/uploads".to_string(),
        toast_action: "/toasts".to_string(),
        upload_limit_bytes: state.upload_limit_bytes,
        upload_limit_mib: state.upload_limit_bytes.div_ceil(1_048_576),
        queue: admin_views::AdminUploadQueueView {
            entries: Vec::new(),
            limit_mib: state.upload_limit_bytes.div_ceil(1_048_576),
        },
    };

    let panel_html =
        match render_upload_form_panel_html(&content, "infra::http::admin_upload_store") {
            Ok(html) => html,
            Err(err) => return err.into_response(),
        };

    let mut stream = datastar_replace(ADMIN_CONTENT, wrap_content(panel_html));
    if let Err(err) = push_toasts(&mut stream, &[toast]) {
        return err.into_response();
    }
    stream.into_response()
}

pub(super) async fn respond_with_upload_panel(
    state: &AdminState,
    filter: &UploadQueryFilter,
    cursor_state: &CursorState,
    cursor: Option<UploadCursor>,
    toasts: &[Toast],
    error_source: &'static str,
    template_source: &'static str,
) -> Response {
    let mut content = match build_upload_list_view(state, filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_upload_error(error_source, err).into_response(),
    };

    apply_upload_pagination_links(&mut content, cursor_state);

    match render_upload_panel_html(&content, template_source) {
        Ok(html) => {
            let mut stream = datastar_replace(UPLOADS_PANEL, html);
            if !toasts.is_empty()
                && let Err(err) = push_toasts(&mut stream, toasts)
            {
                return err.into_response();
            }
            stream.into_response()
        }
        Err(err) => err.into_response(),
    }
}

pub(super) fn respond_with_upload_page(
    content: admin_views::AdminUploadListView,
    toasts: &[Toast],
    _error_source: &'static str,
    template_source: &'static str,
) -> Response {
    let panel_html = match render_upload_panel_html(&content, template_source) {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    let mut stream = datastar_replace(ADMIN_CONTENT, wrap_content(panel_html));
    if !toasts.is_empty()
        && let Err(err) = push_toasts(&mut stream, toasts)
    {
        return err.into_response();
    }

    stream.into_response()
}

pub(super) async fn render_full_upload_panel(state: &AdminState) -> Result<String, HttpError> {
    let filter = UploadQueryFilter::default();
    let cursor_state = CursorState::default();

    let mut content = match build_upload_list_view(state, &filter, None).await {
        Ok(content) => content,
        Err(err) => return Err(admin_upload_error("infra::http::admin_upload_store", err)),
    };
    apply_upload_pagination_links(&mut content, &cursor_state);

    render_upload_panel_html(&content, "infra::http::admin_upload_store")
}

pub(super) fn build_download_response(upload: UploadRecord, bytes: Bytes) -> Response {
    let mut response = Response::new(Body::from(bytes.clone()));
    *response.status_mut() = StatusCode::OK;

    let headers = response.headers_mut();

    if let Ok(value) = HeaderValue::from_str(&upload.content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }

    if let Ok(value) = HeaderValue::from_str(&bytes.len().to_string()) {
        headers.insert(header::CONTENT_LENGTH, value);
    }

    let safe_name = upload.filename.replace('"', "'");
    if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{safe_name}\"")) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }

    response
}
