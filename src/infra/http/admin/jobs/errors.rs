//! Error conversion utilities for jobs admin handlers.

use axum::http::StatusCode;

use crate::application::{admin::jobs::AdminJobError, error::HttpError};
use crate::infra::http::repo_error_to_http;

/// Convert AdminJobError to HttpError for HTTP responses.
pub(super) fn admin_job_error(source: &'static str, err: AdminJobError) -> HttpError {
    match err {
        AdminJobError::NotFound => HttpError::new(
            source,
            StatusCode::NOT_FOUND,
            "Job not found",
            "The requested job does not exist".to_string(),
        ),
        AdminJobError::Repo(repo) => repo_error_to_http(source, repo),
    }
}
