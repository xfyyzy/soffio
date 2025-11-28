mod admin;
pub mod api;
mod middleware;
mod public;

pub use admin::{AdminState, build_admin_router};
pub use api::rate_limit::ApiRateLimiter;
pub use api::{ApiState, build_api_router as build_api_v1_router};
pub use public::{HttpState, build_router};

use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use sqlx::Error as SqlxError;

const DATASTAR_REQUEST_HEADER: &str = "datastar-request";

fn db_health_response(result: Result<(), SqlxError>) -> Response {
    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
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
