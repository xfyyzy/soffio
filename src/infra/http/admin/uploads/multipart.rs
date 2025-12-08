//! Multipart upload payload parsing.

use axum::http::StatusCode;
use axum_extra::extract::Multipart;
use axum_extra::extract::multipart::Field;
use tracing::error;

use super::errors::UploadPayloadError;

const SOURCE_BASE: &str = "infra::http::admin_uploads";

pub(super) struct UploadPayload {
    pub(super) filename: String,
    pub(super) content_type: String,
    pub(super) field: Field,
    pub(super) queue_entry_id: Option<String>,
    pub(super) suppress_panel_patch: bool,
}

pub(super) async fn read_upload_payload(
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
