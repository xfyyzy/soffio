pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod rate_limit;
pub mod state;

pub use state::ApiState;

use axum::{
    Router, middleware as axum_middleware,
    routing::{delete, get, patch, post},
};

use crate::infra::http::RouterState;
use crate::infra::http::middleware::log_responses;

pub fn build_api_router(state: RouterState) -> Router<RouterState> {
    let auth_state = state.clone();
    let rate_state = state.clone();

    Router::new()
        .route(
            "/api/v1/posts",
            get(handlers::list_posts).post(handlers::create_post),
        )
        .route("/api/v1/posts/{id}", patch(handlers::update_post))
        .route(
            "/api/v1/posts/{id}/status",
            post(handlers::update_post_status),
        )
        .route("/api/v1/posts/{id}/tags", post(handlers::replace_post_tags))
        .route("/api/v1/posts/slug/{slug}", get(handlers::get_post))
        .route(
            "/api/v1/pages",
            get(handlers::list_pages).post(handlers::create_page),
        )
        .route("/api/v1/pages/{id}", patch(handlers::update_page))
        .route(
            "/api/v1/pages/{id}/status",
            post(handlers::update_page_status),
        )
        .route("/api/v1/pages/slug/{slug}", get(handlers::get_page))
        .route(
            "/api/v1/tags",
            get(handlers::list_tags).post(handlers::create_tag),
        )
        .route(
            "/api/v1/tags/{id}",
            patch(handlers::update_tag).delete(handlers::delete_tag),
        )
        .route(
            "/api/v1/navigation",
            get(handlers::list_navigation).post(handlers::create_navigation),
        )
        .route(
            "/api/v1/navigation/{id}",
            patch(handlers::update_navigation).delete(handlers::delete_navigation),
        )
        .route(
            "/api/v1/uploads",
            get(handlers::list_uploads).post(handlers::upload_file),
        )
        .route("/api/v1/uploads/{id}", delete(handlers::delete_upload))
        .route(
            "/api/v1/site/settings",
            get(handlers::get_settings).patch(handlers::patch_settings),
        )
        .route("/api/v1/jobs", get(handlers::list_jobs))
        .route("/api/v1/audit", get(handlers::list_audit_logs))
        .with_state(state)
        .layer(axum_middleware::from_fn_with_state(
            rate_state,
            middleware::api_rate_limit,
        ))
        .layer(axum_middleware::from_fn_with_state(
            auth_state,
            middleware::api_auth,
        ))
        .layer(axum_middleware::from_fn(log_responses))
}
