//! Error handling for settings admin handlers.

use axum::http::StatusCode;

use crate::{
    application::{admin::settings::AdminSettingsError, error::HttpError},
    infra::http::repo_error_to_http,
};

pub(super) fn admin_settings_error(source: &'static str, err: AdminSettingsError) -> HttpError {
    match err {
        AdminSettingsError::ConstraintViolation(field) => HttpError::new(
            source,
            StatusCode::BAD_REQUEST,
            "Settings request could not be processed",
            format!("Invalid field `{field}`"),
        ),
        AdminSettingsError::Repo(repo) => repo_error_to_http(source, repo),
    }
}
