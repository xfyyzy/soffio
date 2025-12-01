use crate::application::error::ErrorReport;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub error: ApiErrorMessage,
}

pub mod codes {
    pub const BAD_REQUEST: &str = "bad_request";
    pub const UNAUTHORIZED: &str = "unauthorized";
    pub const FORBIDDEN: &str = "forbidden";
    pub const NOT_FOUND: &str = "not_found";
    pub const RATE_LIMITED: &str = "rate_limited";
    pub const DUPLICATE: &str = "duplicate";
    pub const INVALID_CURSOR: &str = "invalid_cursor";
    pub const INVALID_INPUT: &str = "invalid_input";
    pub const INTEGRITY: &str = "integrity_error";
    pub const DB_TIMEOUT: &str = "db_timeout";
    pub const REPO: &str = "repo_error";
    pub const RENDER: &str = "render_error";
    pub const NAVIGATION: &str = "navigation_error";
    pub const UPLOAD: &str = "upload_error";
    pub const SETTINGS: &str = "settings_error";
    pub const JOBS: &str = "jobs_error";
    pub const TAG_IN_USE: &str = "tag_in_use";
}

#[derive(Debug, Serialize)]
pub struct ApiErrorMessage {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    hint: Option<String>,
}

impl ApiError {
    pub fn new(
        status: StatusCode,
        code: &'static str,
        message: &'static str,
        hint: Option<String>,
    ) -> Self {
        Self {
            status,
            code,
            message,
            hint,
        }
    }

    pub fn bad_request(message: &'static str, hint: Option<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, codes::BAD_REQUEST, message, hint)
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            codes::UNAUTHORIZED,
            "API key required",
            None,
        )
    }

    pub fn forbidden() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            codes::FORBIDDEN,
            "API key lacks required scope",
            None,
        )
    }

    pub fn not_found(message: &'static str) -> Self {
        Self::new(StatusCode::NOT_FOUND, codes::NOT_FOUND, message, None)
    }

    pub fn rate_limited(retry_after: u64) -> Response {
        let body = ApiErrorBody {
            error: ApiErrorMessage {
                code: codes::RATE_LIMITED.to_string(),
                message: "Rate limit exceeded".to_string(),
                hint: Some(format!("Retry after {retry_after} seconds")),
            },
        };
        let mut response = (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response();
        if let Ok(value) = axum::http::HeaderValue::from_str(&retry_after.to_string()) {
            response
                .headers_mut()
                .insert(axum::http::header::RETRY_AFTER, value);
        }
        ErrorReport::from_message(
            "infra::http::api::rate_limit",
            StatusCode::TOO_MANY_REQUESTS,
            format!("rate_limited: retry_after={retry_after}"),
        )
        .attach(&mut response);
        response
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let hint = self.hint.clone();
        let body = ApiErrorBody {
            error: ApiErrorMessage {
                code: self.code.to_string(),
                message: self.message.to_string(),
                hint: self.hint,
            },
        };
        let mut response = (self.status, Json(body)).into_response();
        // Attach a structured report so shared logging middleware can emit rich diagnostics.
        ErrorReport::from_message(
            "infra::http::api",
            self.status,
            format!("{}: {}", self.code, hint.as_deref().unwrap_or(self.message)),
        )
        .attach(&mut response);
        response
    }
}
