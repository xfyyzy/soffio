use axum::http::StatusCode;

use crate::application::{admin::pages::AdminPageError, error::HttpError};
use crate::infra::http::repo_error_to_http;

pub(crate) fn admin_page_error(source: &'static str, err: AdminPageError) -> HttpError {
    match err {
        AdminPageError::ConstraintViolation(field) => HttpError::new(
            source,
            StatusCode::BAD_REQUEST,
            "Page request could not be processed",
            format!("Invalid field `{field}`"),
        ),
        AdminPageError::Render(render) => HttpError::new(
            source,
            StatusCode::BAD_REQUEST,
            "Failed to render page content",
            render.to_string(),
        ),
        AdminPageError::Repo(repo) => repo_error_to_http(source, repo),
    }
}
