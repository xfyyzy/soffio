use std::error::Error as StdError;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::{application::feed::FeedError, domain::error::DomainError, infra::error::InfraError};

#[derive(Debug, Clone)]
pub struct ErrorReport {
    pub source: &'static str,
    pub status: StatusCode,
    pub messages: Vec<String>,
}

impl ErrorReport {
    pub fn from_error(source: &'static str, status: StatusCode, error: &dyn StdError) -> Self {
        let mut messages = Vec::new();
        messages.push(error.to_string());
        let mut current = error.source();
        while let Some(inner) = current {
            messages.push(inner.to_string());
            current = inner.source();
        }
        Self {
            source,
            status,
            messages,
        }
    }

    pub fn from_message(
        source: &'static str,
        status: StatusCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            source,
            status,
            messages: vec![message.into()],
        }
    }

    pub fn attach(self, response: &mut Response) {
        response.extensions_mut().insert(self);
    }
}

#[derive(Debug)]
pub struct HttpError {
    status: StatusCode,
    public_message: &'static str,
    report: ErrorReport,
}

impl HttpError {
    pub fn new(
        source: &'static str,
        status: StatusCode,
        public_message: &'static str,
        detail: impl Into<String>,
    ) -> Self {
        let report = ErrorReport::from_message(source, status, detail);
        Self {
            status,
            public_message,
            report,
        }
    }

    pub fn from_error(
        source: &'static str,
        status: StatusCode,
        public_message: &'static str,
        error: &dyn StdError,
    ) -> Self {
        let report = ErrorReport::from_error(source, status, error);
        Self {
            status,
            public_message,
            report,
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let mut response = (self.status, self.public_message).into_response();
        self.report.attach(&mut response);
        response
    }
}

impl From<FeedError> for HttpError {
    fn from(error: FeedError) -> Self {
        match error {
            FeedError::InvalidCursor(cursor) => HttpError::new(
                "infra::http::feed_error_to_http_error",
                StatusCode::BAD_REQUEST,
                "Invalid cursor",
                format!("Cursor `{cursor}` could not be decoded"),
            ),
            FeedError::UnknownTag => HttpError::new(
                "infra::http::feed_error_to_http_error",
                StatusCode::NOT_FOUND,
                "Unknown tag",
                "Tag filter did not match any known tag",
            ),
            FeedError::UnknownMonth => HttpError::new(
                "infra::http::feed_error_to_http_error",
                StatusCode::NOT_FOUND,
                "Unknown month",
                "Month filter did not match any archive",
            ),
            FeedError::SectionTree(err) => HttpError::from_error(
                "infra::http::feed_error_to_http_error",
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error",
                &err,
            ),
            FeedError::Repo(err) => HttpError::from_error(
                "infra::http::feed_error_to_http_error",
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error",
                &err,
            ),
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Infra(#[from] InfraError),
    #[error("resource not found")]
    NotFound,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("unexpected error: {0}")]
    Unexpected(String),
}

impl AppError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn unexpected(message: impl Into<String>) -> Self {
        Self::Unexpected(message.into())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            AppError::Domain(DomainError::NotFound { .. }) | AppError::NotFound => {
                StatusCode::NOT_FOUND
            }
            AppError::Domain(DomainError::Validation { .. }) | AppError::Validation(_) => {
                StatusCode::BAD_REQUEST
            }
            AppError::Infra(InfraError::Configuration { .. }) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Infra(InfraError::Telemetry(_)) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Infra(InfraError::Database { .. }) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Infra(InfraError::Io(_)) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Domain(DomainError::Invariant { .. }) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn presentation_message(&self) -> &'static str {
        match self {
            AppError::Domain(DomainError::NotFound { .. }) | AppError::NotFound => {
                "Resource not found"
            }
            AppError::Domain(DomainError::Validation { .. }) | AppError::Validation(_) => {
                "Request could not be processed"
            }
            AppError::Infra(InfraError::Database { .. }) => "Service temporarily unavailable",
            AppError::Infra(InfraError::Configuration { .. }) => "Service misconfigured",
            AppError::Infra(InfraError::Telemetry(_)) => "Logging subsystem could not start",
            AppError::Infra(InfraError::Io(_)) => "I/O failure during request",
            AppError::Domain(DomainError::Invariant { .. }) | AppError::Unexpected(_) => {
                "Unexpected error occurred"
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let message = self.presentation_message();
        let report = ErrorReport::from_error("application::error::AppError", status, &self);
        let mut response = (status, message).into_response();
        report.attach(&mut response);
        response
    }
}
