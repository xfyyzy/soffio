use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use super::AdminState;

pub(super) async fn invalidate_cache(State(state): State<AdminState>) -> Response {
    state.cache.invalidate_all().await;
    StatusCode::NO_CONTENT.into_response()
}
