pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod rate_limit;
pub mod state;

pub use state::ApiState;

use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};

use crate::infra::http::RouterState;
use crate::infra::http::middleware::log_responses;

pub fn build_api_router(state: RouterState) -> Router<RouterState> {
    let auth_state = state.clone();
    let rate_state = state.clone();

    Router::new()
        .route("/api/v1/api-keys/me", get(handlers::get_api_key_info))
        .route(
            "/api/v1/posts",
            get(handlers::list_posts).post(handlers::create_post),
        )
        .route(
            "/api/v1/posts/{id}",
            get(handlers::get_post_by_id)
                .patch(handlers::update_post)
                .delete(handlers::delete_post),
        )
        .route("/api/v1/posts/{id}/pin", post(handlers::update_post_pin))
        .route(
            "/api/v1/posts/{id}/title-slug",
            post(handlers::update_post_title_slug),
        )
        .route(
            "/api/v1/posts/{id}/excerpt",
            post(handlers::update_post_excerpt),
        )
        .route("/api/v1/posts/{id}/body", post(handlers::update_post_body))
        .route(
            "/api/v1/posts/{id}/summary",
            post(handlers::update_post_summary),
        )
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
        .route(
            "/api/v1/pages/{id}",
            get(handlers::get_page_by_id)
                .patch(handlers::update_page)
                .delete(handlers::delete_page),
        )
        .route(
            "/api/v1/pages/{id}/title-slug",
            post(handlers::update_page_title_slug),
        )
        .route("/api/v1/pages/{id}/body", post(handlers::update_page_body))
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
            get(handlers::get_tag_by_id)
                .patch(handlers::update_tag)
                .delete(handlers::delete_tag),
        )
        .route("/api/v1/tags/slug/{slug}", get(handlers::get_tag_by_slug))
        .route("/api/v1/tags/{id}/pin", post(handlers::update_tag_pin))
        .route("/api/v1/tags/{id}/name", post(handlers::update_tag_name))
        .route(
            "/api/v1/tags/{id}/description",
            post(handlers::update_tag_description),
        )
        .route(
            "/api/v1/navigation",
            get(handlers::list_navigation).post(handlers::create_navigation),
        )
        .route(
            "/api/v1/navigation/{id}",
            get(handlers::get_navigation_item)
                .patch(handlers::update_navigation)
                .delete(handlers::delete_navigation),
        )
        .route(
            "/api/v1/navigation/{id}/label",
            post(handlers::update_navigation_label),
        )
        .route(
            "/api/v1/navigation/{id}/destination",
            post(handlers::update_navigation_destination),
        )
        .route(
            "/api/v1/navigation/{id}/sort-order",
            post(handlers::update_navigation_sort_order),
        )
        .route(
            "/api/v1/navigation/{id}/visibility",
            post(handlers::update_navigation_visibility),
        )
        .route(
            "/api/v1/navigation/{id}/open-in-new-tab",
            post(handlers::update_navigation_open_in_new_tab),
        )
        .route(
            "/api/v1/uploads",
            get(handlers::list_uploads).post(handlers::upload_file),
        )
        .route(
            "/api/v1/uploads/{id}",
            get(handlers::get_upload).delete(handlers::delete_upload),
        )
        .route(
            "/api/v1/site/settings",
            get(handlers::get_settings).patch(handlers::patch_settings),
        )
        .route("/api/v1/jobs", get(handlers::list_jobs))
        .route("/api/v1/audit", get(handlers::list_audit_logs))
        .with_state(state)
        // Order matters: log runs after auth+rate so principal is available.
        .layer(axum_middleware::from_fn(log_responses))
        .layer(axum_middleware::from_fn_with_state(
            rate_state,
            middleware::api_rate_limit,
        ))
        .layer(axum_middleware::from_fn_with_state(
            auth_state,
            middleware::api_auth,
        ))
        .layer(axum_middleware::from_fn(
            super::middleware::set_request_context,
        ))
}
