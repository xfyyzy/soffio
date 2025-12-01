mod admin;
pub mod api;
mod middleware;
mod public;

pub use admin::{AdminState, build_admin_router};
pub use api::rate_limit::ApiRateLimiter;
pub use api::{ApiState, build_api_router as build_api_v1_router};
pub use public::{HttpState, build_router};

use crate::application::error::ErrorReport;
use crate::application::error::HttpError;
use crate::application::repos::RepoError;
use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use sqlx::Error as SqlxError;

const DATASTAR_REQUEST_HEADER: &str = "datastar-request";

fn db_health_response(result: Result<(), SqlxError>) -> Response {
    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => {
            let mut response = StatusCode::SERVICE_UNAVAILABLE.into_response();
            ErrorReport::from_error(
                "infra::http::db_health",
                StatusCode::SERVICE_UNAVAILABLE,
                &err,
            )
            .attach(&mut response);
            response
        }
    }
}

/// Map a repository error to a consistent HTTP error response for admin/public surfaces.
pub fn repo_error_to_http(source: &'static str, err: RepoError) -> HttpError {
    match err {
        RepoError::Duplicate { constraint } => {
            HttpError::new(source, StatusCode::CONFLICT, "Duplicate record", constraint)
        }
        RepoError::Pagination(p) => HttpError::new(
            source,
            StatusCode::BAD_REQUEST,
            "Invalid cursor",
            p.to_string(),
        ),
        RepoError::NotFound => HttpError::new(
            source,
            StatusCode::NOT_FOUND,
            "Resource not found",
            "resource not found",
        ),
        RepoError::InvalidInput { message } => {
            HttpError::new(source, StatusCode::BAD_REQUEST, "Invalid input", message)
        }
        RepoError::Integrity { message } => HttpError::new(
            source,
            StatusCode::CONFLICT,
            "Integrity constraint violated",
            message,
        ),
        RepoError::Timeout => HttpError::new(
            source,
            StatusCode::SERVICE_UNAVAILABLE,
            "Database timeout",
            "Database timeout",
        ),
        RepoError::Persistence(message) => HttpError::new(
            source,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Persistence error",
            message,
        ),
    }
}

#[derive(Clone)]
pub struct RouterState {
    pub http: HttpState,
    pub api: ApiState,
}

impl FromRef<RouterState> for HttpState {
    fn from_ref(state: &RouterState) -> Self {
        state.http.clone()
    }
}

impl FromRef<RouterState> for ApiState {
    fn from_ref(state: &RouterState) -> Self {
        state.api.clone()
    }
}
