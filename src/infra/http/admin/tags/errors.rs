use axum::http::StatusCode;

use crate::application::{admin::tags::AdminTagError, error::HttpError};
use crate::infra::http::repo_error_to_http;

pub(super) fn admin_tag_error(source: &'static str, err: AdminTagError) -> HttpError {
    match err {
        AdminTagError::ConstraintViolation(field) => HttpError::new(
            source,
            StatusCode::BAD_REQUEST,
            "Tag request could not be processed",
            format!("Invalid field `{field}`"),
        ),
        AdminTagError::InUse { count } => HttpError::new(
            source,
            StatusCode::BAD_REQUEST,
            "Tag request could not be processed",
            format!("Tag is referenced by {count} posts"),
        ),
        AdminTagError::Repo(repo) => repo_error_to_http(source, repo),
    }
}
