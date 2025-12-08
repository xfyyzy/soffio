//! Error handling utilities for upload handlers.

use axum::http::StatusCode;

use crate::{
    application::{admin::uploads::AdminUploadError, error::HttpError},
    infra::http::repo_error_to_http,
};

use super::super::shared::Toast;

pub(super) enum UploadPayloadError {
    Missing,
    PayloadTooLarge,
    InvalidFormData,
    Read { _detail: String },
}

impl UploadPayloadError {
    pub(super) fn into_toast(self, limit_bytes: u64) -> Toast {
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

pub(super) fn admin_upload_error(source: &'static str, err: AdminUploadError) -> HttpError {
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
