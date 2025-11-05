use axum::http::StatusCode;

use crate::application::{admin::posts::AdminPostError, error::HttpError};

pub(super) fn admin_post_error(source: &'static str, err: AdminPostError) -> HttpError {
    match err {
        AdminPostError::ConstraintViolation(field) => HttpError::new(
            source,
            StatusCode::BAD_REQUEST,
            "Post request could not be processed",
            format!("Invalid field `{field}`"),
        ),
        AdminPostError::Repo(repo) => HttpError::from_error(
            source,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error",
            &repo,
        ),
    }
}
