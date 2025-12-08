//! HTTP handlers for upload admin.

use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::Multipart;
use tracing::{error, warn};
use uuid::Uuid;

use crate::{
    application::{
        admin::uploads::AdminUploadError, error::HttpError, pagination::UploadCursor,
        repos::UploadQueryFilter,
    },
    presentation::{admin::views as admin_views, views::render_template_response},
};

use super::super::{
    AdminState,
    pagination::CursorState,
    selectors::UPLOAD_QUEUE_BODY,
    shared::{Toast, datastar_replace},
};

use super::errors::admin_upload_error;
use super::forms::{
    AdminUploadDeleteForm, AdminUploadPanelForm, AdminUploadQuery, UploadQueuePreviewForm,
};
use super::multipart::read_upload_payload;
use super::panel::{
    apply_upload_pagination_links, build_upload_filter, build_upload_list_view,
    render_upload_queue_html,
};
use super::queue::parse_queue_manifest;
use super::response::{build_download_response, respond_with_upload_panel};
use super::storage::{handle_upload_payload, upload_payload_error};

const SOURCE_BASE: &str = "infra::http::admin_uploads";

pub(crate) async fn admin_uploads(
    State(state): State<AdminState>,
    Query(query): Query<AdminUploadQuery>,
) -> Response {
    let chrome = match state.chrome.load("/uploads").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(query.cursor.clone(), query.trail.clone());
    let cursor = match cursor_state.decode_with(UploadCursor::decode, SOURCE_BASE) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = build_upload_filter(
        query.search.as_deref(),
        query.content_type.as_deref(),
        query.month.as_deref(),
    );

    let mut content = match build_upload_list_view(&state, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_upload_error(SOURCE_BASE, err).into_response(),
    };

    apply_upload_pagination_links(&mut content, &cursor_state);

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminUploadsTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_uploads_panel(
    State(state): State<AdminState>,
    Form(form): Form<AdminUploadPanelForm>,
) -> Response {
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let cursor = match cursor_state.decode_with(UploadCursor::decode, SOURCE_BASE) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = if form.clear.is_some() {
        UploadQueryFilter::default()
    } else {
        build_upload_filter(
            form.search.as_deref(),
            form.content_type.as_deref(),
            form.month.as_deref(),
        )
    };

    respond_with_upload_panel(
        &state,
        &filter,
        &cursor_state,
        cursor,
        &[],
        "infra::http::admin_uploads_panel",
        "infra::http::admin_uploads_panel",
    )
    .await
}

pub(crate) async fn admin_upload_queue_preview(
    State(state): State<AdminState>,
    Form(form): Form<UploadQueuePreviewForm>,
) -> Response {
    let queue = match parse_queue_manifest(&form.queue_manifest) {
        Ok(entries) => admin_views::AdminUploadQueueView {
            entries,
            limit_mib: state.upload_limit_bytes.div_ceil(1_048_576),
        },
        Err(err) => return err.into_response(),
    };

    let html = match render_upload_queue_html(&queue, "infra::http::admin_upload_queue_preview") {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    let stream = datastar_replace(UPLOAD_QUEUE_BODY, html);
    stream.into_response()
}

pub(crate) async fn admin_upload_new(State(state): State<AdminState>) -> Response {
    let chrome = match state.chrome.load("/uploads").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

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

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminUploadNewTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_upload_store(
    State(state): State<AdminState>,
    mut multipart: Multipart,
) -> Response {
    match read_upload_payload(&mut multipart).await {
        Ok(payload) => handle_upload_payload(&state, payload).await,
        Err(err) => upload_payload_error(&state, err).await,
    }
}

pub(crate) async fn admin_upload_delete(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminUploadDeleteForm>,
) -> Response {
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let cursor = match cursor_state.decode_with(UploadCursor::decode, SOURCE_BASE) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = build_upload_filter(
        form.search.as_deref(),
        form.content_type.as_deref(),
        form.month.as_deref(),
    );

    let actor = "admin";
    let result = state.uploads.delete_upload(actor, id).await;

    match result {
        Ok(record) => {
            if let Err(err) = state.upload_storage.delete(&record.stored_path).await {
                warn!(
                    target = SOURCE_BASE,
                    upload_id = %record.id,
                    error = %err,
                    "failed to remove stored payload during delete"
                );
            }

            let toasts = [Toast::success("Upload deleted")];
            respond_with_upload_panel(
                &state,
                &filter,
                &cursor_state,
                cursor,
                &toasts,
                "infra::http::admin_upload_delete",
                "infra::http::admin_upload_delete",
            )
            .await
        }
        Err(AdminUploadError::NotFound) => {
            let toasts = [Toast::error("Upload record not found")];
            respond_with_upload_panel(
                &state,
                &filter,
                &cursor_state,
                cursor,
                &toasts,
                "infra::http::admin_upload_delete",
                "infra::http::admin_upload_delete",
            )
            .await
        }
        Err(AdminUploadError::Repo(err)) => admin_upload_error(
            "infra::http::admin_upload_delete",
            AdminUploadError::Repo(err),
        )
        .into_response(),
    }
}

pub(crate) async fn admin_upload_download(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
) -> Response {
    let upload = match state.uploads.find_upload(id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return admin_upload_error(
                "infra::http::admin_upload_download",
                AdminUploadError::NotFound,
            )
            .into_response();
        }
        Err(err) => {
            return admin_upload_error("infra::http::admin_upload_download", err).into_response();
        }
    };

    match state.upload_storage.read(&upload.stored_path).await {
        Ok(bytes) => build_download_response(upload, bytes),
        Err(err) => {
            error!(
                target = SOURCE_BASE,
                upload_id = %upload.id,
                error = %err,
                "failed to read stored upload"
            );
            HttpError::new(
                "infra::http::admin_upload_download",
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read uploaded file",
                err.to_string(),
            )
            .into_response()
        }
    }
}
