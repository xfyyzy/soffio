use axum::{extract::State, response::Response};

use super::super::db_health_response;
use super::AdminState;

pub(super) async fn admin_health(State(state): State<AdminState>) -> Response {
    db_health_response(state.db.health_check().await)
}
