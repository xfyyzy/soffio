//! Queue-related functions for upload admin.

use axum::http::StatusCode;
use serde_json::json;

use crate::{application::error::HttpError, util::bytes::format_bytes};

use super::super::AdminState;
use super::super::shared::Toast;
use super::forms::UploadQueueManifestEntry;
use super::response::respond_with_upload_form;
use crate::application::stream::StreamBuilder;
use crate::presentation::admin::views as admin_views;
use axum::response::Response;

pub(super) const UPLOAD_QUEUE_EVENT: &str = "admin:upload-entry";

pub(super) fn parse_queue_manifest(
    raw: &str,
) -> Result<Vec<admin_views::AdminUploadQueueEntry>, HttpError> {
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

pub(super) fn queue_status_label(status: &str) -> String {
    match status {
        "pending" => "Ready",
        "uploading" => "Uploading…",
        "success" => "Uploaded",
        "error" => "Failed",
        other => other,
    }
    .to_string()
}

pub(super) async fn respond_with_queue_error_or_form(
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

pub(super) fn push_queue_event(
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
