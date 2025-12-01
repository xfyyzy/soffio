use axum::http::StatusCode;

use crate::application::{admin::posts::AdminPostError, error::HttpError};
use crate::infra::http::repo_error_to_http;

pub(super) fn admin_post_error(source: &'static str, err: AdminPostError) -> HttpError {
    match err {
        AdminPostError::ConstraintViolation(field) => HttpError::new(
            source,
            StatusCode::BAD_REQUEST,
            "Post request could not be processed",
            format!("Invalid field `{field}`"),
        ),
        AdminPostError::Repo(repo) => repo_error_to_http(source, repo),
    }
}
