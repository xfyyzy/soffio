use crate::application::stream::StreamBuilder;
use askama::Template;
use axum::{
    body::Body,
    extract::{Form, Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Multipart;
use axum_extra::extract::multipart::{Field, MultipartError};
use bytes::Bytes;
use datastar::prelude::ElementPatchMode;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{self, json};
use std::convert::TryFrom;
use time::OffsetDateTime;
use tracing::{error, warn};
use url::form_urlencoded;
use uuid::Uuid;

use crate::{
    application::{
        admin::uploads::AdminUploadError,
        error::HttpError,
        metadata::metadata_registry,
        pagination::UploadCursor,
        repos::{RepoError, SettingsRepo, UploadQueryFilter},
    },
    domain::{
        entities::UploadRecord,
        uploads::{self, UploadMetadata},
    },
    infra::http::repo_error_to_http,
    infra::uploads::UploadStorageError,
    presentation::{admin::views as admin_views, views::render_template_response},
    util::bytes::format_bytes,
};

use super::{
    AdminState,
    pagination::{self, CursorState},
    selectors::{ADMIN_CONTENT, UPLOAD_QUEUE_BODY, UPLOADS_PANEL},
    shared::{Toast, blank_to_none_opt, datastar_replace, push_toasts, template_render_http_error},
};

const SOURCE_BASE: &str = "infra::http::admin_uploads";
const UPLOAD_QUEUE_EVENT: &str = "admin:upload-entry";

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct AdminUploadQuery {
    cursor: Option<String>,
    trail: Option<String>,
    search: Option<String>,
    #[serde(rename = "content_type")]
    content_type: Option<String>,
    month: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminUploadPanelForm {
    pub search: Option<String>,
    #[serde(rename = "content_type")]
    pub content_type: Option<String>,
    pub month: Option<String>,
    pub cursor: Option<String>,
    pub trail: Option<String>,
    pub clear: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminUploadDeleteForm {
    pub cursor: Option<String>,
    pub trail: Option<String>,
    pub search: Option<String>,
    #[serde(rename = "content_type")]
    pub content_type: Option<String>,
    pub month: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct UploadQueuePreviewForm {
    #[serde(default)]
    pub queue_manifest: String,
}

#[derive(Debug, Deserialize)]
struct UploadQueueManifestEntry {
    pub id: Option<String>,
    pub filename: Option<String>,
    pub size_bytes: Option<u64>,
    pub status: Option<String>,
    pub message: Option<String>,
}

pub(super) async fn admin_uploads(
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

pub(super) async fn admin_uploads_panel(
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

pub(super) async fn admin_upload_queue_preview(
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

pub(super) async fn admin_upload_new(State(state): State<AdminState>) -> Response {
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

pub(super) async fn admin_upload_store(
    State(state): State<AdminState>,
    mut multipart: Multipart,
) -> Response {
    match read_upload_payload(&mut multipart).await {
        Ok(payload) => handle_upload_payload(&state, payload).await,
        Err(err) => upload_payload_error(&state, err).await,
    }
}

pub(super) async fn admin_upload_delete(
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

pub(super) async fn admin_upload_download(
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

async fn handle_upload_payload(state: &AdminState, payload: UploadPayload) -> Response {
    let UploadPayload {
        filename,
        content_type,
        field,
        queue_entry_id,
        suppress_panel_patch,
    } = payload;

    let entry_id = queue_entry_id.as_deref();

    let stream = field.map(|result| {
        result.map_err(|err| {
            if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
                UploadStorageError::PayloadTooLarge {
                    source: Box::new(err),
                }
            } else {
                UploadStorageError::PayloadStream {
                    source: Box::new(err),
                }
            }
        })
    });

    let limit_bytes = state.upload_limit_bytes;

    let stored = match state.upload_storage.store_stream(&filename, stream).await {
        Ok(stored) => stored,
        Err(UploadStorageError::EmptyPayload) => {
            return respond_with_queue_error_or_form(
                state,
                entry_id,
                "Uploaded file is empty".to_string(),
                suppress_panel_patch,
            )
            .await;
        }
        Err(UploadStorageError::PayloadTooLarge { source }) => {
            let limit_mib = limit_bytes.div_ceil(1_048_576);
            error!(
                target = SOURCE_BASE,
                error = %source,
                limit_bytes = limit_bytes,
                limit_mib = limit_mib,
                "upload request exceeded configured body limit"
            );
            return respond_with_queue_error_or_form(
                state,
                entry_id,
                format!("File is too large (limit is {limit_mib} MiB)"),
                suppress_panel_patch,
            )
            .await;
        }
        Err(UploadStorageError::SizeOverflow) => {
            let limit_mib = limit_bytes.div_ceil(1_048_576);
            error!(
                target = SOURCE_BASE,
                limit_bytes = limit_bytes,
                limit_mib = limit_mib,
                "upload stream size exceeded supported range"
            );
            return respond_with_queue_error_or_form(
                state,
                entry_id,
                format!("File is too large (limit is {limit_mib} MiB)"),
                suppress_panel_patch,
            )
            .await;
        }
        Err(UploadStorageError::PayloadStream { source }) => {
            let message = if let Some(multipart_err) = source.downcast_ref::<MultipartError>() {
                match multipart_err.status() {
                    StatusCode::PAYLOAD_TOO_LARGE => {
                        let limit_mib = limit_bytes.div_ceil(1_048_576);
                        format!("File is too large (limit is {limit_mib} MiB)")
                    }
                    StatusCode::BAD_REQUEST => "Upload form data was invalid".to_string(),
                    _ => "Could not store uploaded file, please retry later".to_string(),
                }
            } else {
                "Could not store uploaded file, please retry later".to_string()
            };

            error!(
                target = SOURCE_BASE,
                error = %source,
                "failed to persist upload payload"
            );
            return respond_with_queue_error_or_form(
                state,
                entry_id,
                message,
                suppress_panel_patch,
            )
            .await;
        }
        Err(err) => {
            error!(
                target = SOURCE_BASE,
                error = %err,
                "failed to persist upload payload"
            );
            return respond_with_queue_error_or_form(
                state,
                entry_id,
                "Could not store uploaded file, please retry later".to_string(),
                suppress_panel_patch,
            )
            .await;
        }
    };

    let stored_size = u64::try_from(stored.size_bytes).unwrap_or(0);

    let metadata = match state.upload_storage.absolute_path(&stored.stored_path) {
        Ok(path) => match metadata_registry().extract_from_file(&content_type, path.as_path()) {
            Ok(metadata) => metadata,
            Err(err) => {
                warn!(
                    target = SOURCE_BASE,
                    error = %err,
                    "failed to extract metadata from uploaded asset"
                );
                UploadMetadata::new()
            }
        },
        Err(err) => {
            warn!(
                target = SOURCE_BASE,
                error = %err,
                "failed to resolve stored upload path for metadata extraction"
            );
            UploadMetadata::new()
        }
    };

    let record = UploadRecord {
        id: Uuid::new_v4(),
        filename: filename.clone(),
        content_type,
        size_bytes: stored.size_bytes,
        checksum: stored.checksum.clone(),
        stored_path: stored.stored_path.clone(),
        metadata,
        created_at: OffsetDateTime::now_utc(),
    };

    let actor = "admin";
    match state.uploads.register_upload(actor, record.clone()).await {
        Ok(_) => {
            if let Some(id) = entry_id {
                let mut stream = StreamBuilder::new();
                push_queue_event(
                    &mut stream,
                    Some(id),
                    "success",
                    None,
                    Some(stored_size),
                    suppress_panel_patch,
                );

                if !suppress_panel_patch && let Ok(html) = render_full_upload_panel(state).await {
                    stream.push_patch(html, UPLOADS_PANEL, ElementPatchMode::Replace);
                }

                return stream.into_response();
            }

            let filter = UploadQueryFilter::default();
            let cursor_state = CursorState::default();
            match build_upload_list_view(state, &filter, None).await {
                Ok(mut content) => {
                    apply_upload_pagination_links(&mut content, &cursor_state);
                    let toasts = [Toast::success("File uploaded successfully")];
                    respond_with_upload_page(
                        content,
                        &toasts,
                        "infra::http::admin_upload_store",
                        "infra::http::admin_upload_store",
                    )
                }
                Err(err) => {
                    admin_upload_error("infra::http::admin_upload_store", err).into_response()
                }
            }
        }
        Err(AdminUploadError::Repo(repo_err)) => {
            let message = match &repo_err {
                RepoError::Duplicate { constraint } => {
                    error!(
                        target = SOURCE_BASE,
                        error = %repo_err,
                        constraint = constraint.as_str(),
                        "duplicate upload detected while registering metadata"
                    );
                    "This file was already uploaded".to_string()
                }
                _ => {
                    error!(
                        target = SOURCE_BASE,
                        error = %repo_err,
                        "failed to register upload metadata"
                    );
                    "Could not save upload record, please retry later".to_string()
                }
            };

            if let Err(remove_err) = state.upload_storage.delete(&record.stored_path).await {
                warn!(
                    target = SOURCE_BASE,
                    error = %remove_err,
                    "failed to roll back stored upload after persistence error"
                );
            }

            if let Some(id) = entry_id {
                let mut stream = StreamBuilder::new();
                push_queue_event(
                    &mut stream,
                    Some(id),
                    "error",
                    Some(&message),
                    Some(stored_size),
                    suppress_panel_patch,
                );
                return stream.into_response();
            }

            respond_with_upload_form(state, Toast::error(message)).await
        }
        Err(AdminUploadError::NotFound) => {
            unreachable!("register_upload cannot yield NotFound")
        }
    }
}

async fn upload_payload_error(state: &AdminState, err: UploadPayloadError) -> Response {
    respond_with_upload_form(state, err.into_toast(state.upload_limit_bytes)).await
}

async fn respond_with_upload_form(state: &AdminState, toast: Toast) -> Response {
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

async fn respond_with_upload_panel(
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

fn respond_with_upload_page(
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

async fn render_full_upload_panel(state: &AdminState) -> Result<String, HttpError> {
    let filter = UploadQueryFilter::default();
    let cursor_state = CursorState::default();

    let mut content = match build_upload_list_view(state, &filter, None).await {
        Ok(content) => content,
        Err(err) => return Err(admin_upload_error("infra::http::admin_upload_store", err)),
    };
    apply_upload_pagination_links(&mut content, &cursor_state);

    render_upload_panel_html(&content, "infra::http::admin_upload_store")
}

fn build_download_response(upload: UploadRecord, bytes: Bytes) -> Response {
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

fn wrap_content(inner_html: String) -> String {
    format!("<div data-admin-content>{inner_html}</div>")
}

fn build_upload_filter(
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

async fn build_upload_list_view(
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
    })
}

fn render_upload_panel_html(
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

fn render_upload_form_panel_html(
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

fn render_upload_queue_html(
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

fn map_upload_row(
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

fn apply_upload_pagination_links(
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

fn normalize_public_site_url(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    }
}

fn parse_queue_manifest(raw: &str) -> Result<Vec<admin_views::AdminUploadQueueEntry>, HttpError> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    let manifest: Vec<UploadQueueManifestEntry> = serde_json::from_str(raw).map_err(|_| {
        HttpError::new(
            "infra::http::admin_upload_queue_preview",
            StatusCode::BAD_REQUEST,
            "Invalid upload queue",
            format!("Queue manifest could not be parsed (length {})", raw.len()),
        )
    })?;

    manifest
        .into_iter()
        .map(|entry| {
            let filename = entry.filename.unwrap_or_default().trim().to_string();
            if filename.is_empty() {
                return Err(HttpError::new(
                    "infra::http::admin_upload_queue_preview",
                    StatusCode::BAD_REQUEST,
                    "Invalid upload queue",
                    "Queue entries must include a filename",
                ));
            }

            let status = entry
                .status
                .unwrap_or_else(|| "pending".to_string())
                .trim()
                .to_string();

            let size_bytes = entry.size_bytes.unwrap_or(0);
            let message = entry.message.and_then(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            });

            Ok(admin_views::AdminUploadQueueEntry {
                id: entry.id.and_then(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                }),
                filename,
                size_bytes,
                size_label: format_bytes(size_bytes),
                status: status.clone(),
                status_label: queue_status_label(&status),
                message,
            })
        })
        .collect()
}

fn queue_status_label(status: &str) -> String {
    match status {
        "pending" => "Ready",
        "uploading" => "Uploading…",
        "success" => "Uploaded",
        "error" => "Failed",
        other => other,
    }
    .to_string()
}

async fn respond_with_queue_error_or_form(
    state: &AdminState,
    entry_id: Option<&str>,
    message: String,
    suppress_panel_patch: bool,
) -> Response {
    if let Some(id) = entry_id {
        let mut stream = StreamBuilder::new();
        push_queue_event(
            &mut stream,
            Some(id),
            "error",
            Some(&message),
            None,
            suppress_panel_patch,
        );
        return stream.into_response();
    }

    respond_with_upload_form(state, Toast::error(message)).await
}

fn push_queue_event(
    stream: &mut StreamBuilder,
    entry_id: Option<&str>,
    status: &str,
    message: Option<&str>,
    size_bytes: Option<u64>,
    suppress_panel_patch: bool,
) {
    if let Some(id) = entry_id {
        let detail = json!({
            "id": id,
            "status": status,
            "message": message,
            "sizeBytes": size_bytes,
            "suppressPanel": suppress_panel_patch,
        });
        stream.push_script(format!(
            "window.dispatchEvent(new CustomEvent('{UPLOAD_QUEUE_EVENT}', {{ detail: {detail} }}));"
        ));
    }
}

struct UploadPayload {
    filename: String,
    content_type: String,
    field: Field,
    queue_entry_id: Option<String>,
    suppress_panel_patch: bool,
}

async fn read_upload_payload(
    multipart: &mut Multipart,
) -> Result<UploadPayload, UploadPayloadError> {
    let mut queue_entry_id: Option<String> = None;
    let mut suppress_panel_patch = false;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                match field.name() {
                    Some("queue_entry_id") => {
                        let value = field
                            .text()
                            .await
                            .map_err(|_| UploadPayloadError::InvalidFormData)?
                            .trim()
                            .to_string();
                        if !value.is_empty() {
                            queue_entry_id = Some(value);
                        }
                        continue;
                    }
                    Some("suppress_panel_patch") => {
                        let value = field
                            .text()
                            .await
                            .map_err(|_| UploadPayloadError::InvalidFormData)?
                            .trim()
                            .to_ascii_lowercase();
                        suppress_panel_patch =
                            matches!(value.as_str(), "true" | "1" | "yes" | "on");
                        continue;
                    }
                    Some("file") => {}
                    _ => continue,
                }

                let filename = field
                    .file_name()
                    .map(|value| value.to_string())
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "upload.bin".to_string());

                let content_type = field
                    .content_type()
                    .map(|mime| mime.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string());

                return Ok(UploadPayload {
                    filename,
                    content_type,
                    field,
                    queue_entry_id,
                    suppress_panel_patch,
                });
            }
            Ok(None) => break,
            Err(err) => {
                let status = err.status();
                error!(
                    target = SOURCE_BASE,
                    status = status.as_u16(),
                    error = %err,
                    "failed to read multipart payload"
                );
                return Err(match status {
                    StatusCode::PAYLOAD_TOO_LARGE => UploadPayloadError::PayloadTooLarge,
                    StatusCode::BAD_REQUEST => UploadPayloadError::InvalidFormData,
                    _ => UploadPayloadError::Read {
                        _detail: err.to_string(),
                    },
                });
            }
        }
    }

    Err(UploadPayloadError::Missing)
}

enum UploadPayloadError {
    Missing,
    PayloadTooLarge,
    InvalidFormData,
    Read { _detail: String },
}

impl UploadPayloadError {
    fn into_toast(self, limit_bytes: u64) -> Toast {
        match self {
            UploadPayloadError::Missing => Toast::error("Please choose a file to upload"),
            UploadPayloadError::PayloadTooLarge => {
                let limit_mib = limit_bytes.div_ceil(1_048_576);
                Toast::error(format!("File is too large (limit is {limit_mib} MiB)"))
            }
            UploadPayloadError::InvalidFormData => Toast::error("Upload form data was invalid"),
            UploadPayloadError::Read { .. } => Toast::error("Upload failed, please try again"),
        }
    }
}

fn admin_upload_error(source: &'static str, err: AdminUploadError) -> HttpError {
    match err {
        AdminUploadError::NotFound => HttpError::new(
            source,
            StatusCode::NOT_FOUND,
            "Upload not found",
            "The requested upload does not exist",
        ),
        AdminUploadError::Repo(repo) => repo_error_to_http(source, repo),
    }
}
