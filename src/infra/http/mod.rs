mod admin;
mod middleware;
mod public;

pub use admin::{AdminState, build_admin_router};
pub use public::{HttpState, build_router};

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
