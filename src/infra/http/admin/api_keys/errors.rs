use axum::{http::StatusCode, response::{IntoResponse, Response}};

use crate::application::api_keys::ApiKeyError;

#[derive(Debug)]
pub struct ApiKeyHttpError(ApiErrorKind);

#[derive(Debug)]
enum ApiErrorKind {
    BadRequest(&'static str, Option<String>),
    Service(String),
}

impl ApiKeyHttpError {
    pub fn bad_request(message: &'static str) -> Self {
        Self(ApiErrorKind::BadRequest(message, None))
    }

    pub fn from_api(err: ApiKeyError) -> Self {
        match err {
            ApiKeyError::InvalidScopes => Self::bad_request("invalid scopes"),
            ApiKeyError::NotFound => Self::bad_request("key not found"),
            ApiKeyError::Repo(repo) => Self(ApiErrorKind::Service(repo.to_string())),
        }
    }

    pub fn from_template(err: askama::Error, source: &'static str) -> Self {
        Self(ApiErrorKind::Service(format!(
            "{source} template error: {err}"
        )))
    }

    pub fn service(message: impl Into<String>) -> Self {
        Self(ApiErrorKind::Service(message.into()))
    }

    pub fn from_repo(err: crate::application::repos::RepoError) -> Self {
        Self(ApiErrorKind::Service(err.to_string()))
    }

    pub fn from_http(err: impl std::fmt::Debug) -> Self {
        Self::service(format!("{err:?}"))
    }
}

impl IntoResponse for ApiKeyHttpError {
    fn into_response(self) -> Response {
        match self.0 {
            ApiErrorKind::BadRequest(message, hint) => crate::application::error::HttpError::new(
                "infra::http::admin_api_keys",
                StatusCode::BAD_REQUEST,
                message,
                hint.unwrap_or_else(|| "Invalid request".to_string()),
            )
            .into_response(),
            ApiErrorKind::Service(detail) => crate::application::error::HttpError::new(
                "infra::http::admin_api_keys",
                StatusCode::INTERNAL_SERVER_ERROR,
                "API key operation failed",
                detail,
            )
            .into_response(),
        }
    }
}
