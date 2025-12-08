//! Upload storage processing logic.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::multipart::MultipartError;
use datastar::prelude::ElementPatchMode;
use futures::StreamExt;
use std::convert::TryFrom;
use time::OffsetDateTime;
use tracing::{error, warn};
use uuid::Uuid;

use crate::{
    application::{
        admin::uploads::AdminUploadError,
        metadata::metadata_registry,
        repos::{RepoError, UploadQueryFilter},
        stream::StreamBuilder,
    },
    domain::{entities::UploadRecord, uploads::UploadMetadata},
    infra::uploads::UploadStorageError,
};

use super::super::{AdminState, pagination::CursorState, selectors::UPLOADS_PANEL, shared::Toast};

use super::errors::{UploadPayloadError, admin_upload_error};
use super::multipart::UploadPayload;
use super::panel::{apply_upload_pagination_links, build_upload_list_view};
use super::queue::{push_queue_event, respond_with_queue_error_or_form};
use super::response::{
    render_full_upload_panel, respond_with_upload_form, respond_with_upload_page,
};

const SOURCE_BASE: &str = "infra::http::admin_uploads";

pub(super) async fn handle_upload_payload(state: &AdminState, payload: UploadPayload) -> Response {
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

pub(super) async fn upload_payload_error(state: &AdminState, err: UploadPayloadError) -> Response {
    respond_with_upload_form(state, err.into_toast(state.upload_limit_bytes)).await
}
