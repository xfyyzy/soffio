mod api_keys;
mod audit;
mod cache;
mod dashboard;
mod health;
mod jobs;
mod navigation;
mod pages;
mod pagination;
mod posts;
mod selectors;
mod settings;
mod shared;
mod state;
mod tags;
mod toasts;
mod uploads;

pub use state::AdminState;

use axum::{
    Router,
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::{
        StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};

use crate::{application::repos::SettingsRepo, infra::assets};

use super::middleware::{invalidate_admin_writes, log_responses};
use tracing::error;

pub fn build_admin_router(state: AdminState, upload_body_limit: usize) -> Router {
    let response_cache = state.cache.clone();
    Router::new()
        .route("/", get(dashboard::admin_dashboard))
        .route("/posts", get(posts::admin_posts))
        .route("/posts/panel", post(posts::admin_posts_panel))
        .route("/posts/bulk", post(posts::admin_posts_bulk_action))
        .route("/posts/create", post(posts::admin_post_create))
        .route("/posts/new", get(posts::admin_post_new))
        .route(
            "/posts/{id}/edit",
            get(posts::admin_post_edit).post(posts::admin_post_update),
        )
        .route(
            "/posts/new/tags/toggle",
            post(posts::admin_post_tags_toggle_new),
        )
        .route(
            "/posts/{id}/tags/toggle",
            post(posts::admin_post_tags_toggle),
        )
        .route("/posts/{id}/delete", post(posts::admin_post_delete))
        .route("/posts/{id}/publish", post(posts::admin_post_publish))
        .route("/posts/{id}/draft", post(posts::admin_post_move_to_draft))
        .route("/posts/{id}/archive", post(posts::admin_post_archive))
        .route("/posts/{id}/pin", post(posts::admin_post_pin))
        .route("/posts/{id}/unpin", post(posts::admin_post_unpin))
        .route("/pages", get(pages::admin_pages))
        .route("/pages/panel", post(pages::admin_page_panel))
        .route("/pages/create", post(pages::admin_page_create))
        .route("/pages/new", get(pages::admin_page_new))
        .route(
            "/pages/{id}/edit",
            get(pages::admin_page_edit).post(pages::admin_page_update),
        )
        .route("/pages/{id}/delete", post(pages::admin_page_delete))
        .route("/pages/{id}/publish", post(pages::admin_page_publish))
        .route("/pages/{id}/draft", post(pages::admin_page_move_to_draft))
        .route("/pages/{id}/archive", post(pages::admin_page_archive))
        .route("/tags", get(tags::admin_tags))
        .route("/tags/panel", post(tags::admin_tags_panel))
        .route("/tags/new", get(tags::admin_tag_new))
        .route("/tags/create", post(tags::admin_tag_create))
        .route(
            "/tags/{id}/edit",
            get(tags::admin_tag_edit).post(tags::admin_tag_update),
        )
        .route("/tags/{id}/pin", post(tags::admin_tag_pin))
        .route("/tags/{id}/unpin", post(tags::admin_tag_unpin))
        .route("/tags/{id}/delete", post(tags::admin_tag_delete))
        .route("/navigation", get(navigation::admin_navigation))
        .route(
            "/navigation/panel",
            post(navigation::admin_navigation_panel),
        )
        .route("/navigation/new", get(navigation::admin_navigation_new))
        .route(
            "/navigation/create",
            post(navigation::admin_navigation_create),
        )
        .route(
            "/navigation/{id}/edit",
            get(navigation::admin_navigation_edit).post(navigation::admin_navigation_update),
        )
        .route(
            "/navigation/{id}/visibility",
            post(navigation::admin_navigation_toggle_visibility),
        )
        .route(
            "/navigation/destination-preview",
            post(navigation::admin_navigation_destination_preview),
        )
        .route(
            "/navigation/{id}/destination-preview",
            post(navigation::admin_navigation_destination_preview_for_item),
        )
        .route(
            "/navigation/{id}/delete",
            post(navigation::admin_navigation_delete),
        )
        .route("/settings", get(settings::admin_settings))
        .route(
            "/settings/edit",
            get(settings::admin_settings_edit).post(settings::admin_settings_update),
        )
        .route(
            "/uploads",
            get(uploads::admin_uploads)
                .post(uploads::admin_upload_store)
                .layer(DefaultBodyLimit::max(upload_body_limit)),
        )
        .route("/uploads/panel", post(uploads::admin_uploads_panel))
        .route(
            "/uploads/queue/preview",
            post(uploads::admin_upload_queue_preview),
        )
        .route("/uploads/new", get(uploads::admin_upload_new))
        .route("/uploads/{id}", get(uploads::admin_upload_download))
        .route("/uploads/{id}/delete", post(uploads::admin_upload_delete))
        .route("/toasts", post(toasts::admin_toast))
        .route("/jobs", get(jobs::admin_jobs))
        .route("/jobs/{id}", get(jobs::admin_job_detail))
        .route("/jobs/{id}/retry", post(jobs::admin_job_retry))
        .route("/jobs/{id}/cancel", post(jobs::admin_job_cancel))
        .route("/audit", get(audit::admin_audit))
        .route("/api-keys", get(api_keys::admin_api_keys))
        .route("/api-keys/create", post(api_keys::admin_api_key_create))
        .route("/api-keys/panel", post(api_keys::admin_api_keys_panel))
        .route("/api-keys/new", get(api_keys::admin_api_key_new))
        .route(
            "/api-keys/{id}/edit",
            get(api_keys::admin_api_key_edit).post(api_keys::admin_api_key_update),
        )
        .route(
            "/api-keys/new/scopes/toggle",
            post(api_keys::admin_api_key_scopes_toggle),
        )
        .route(
            "/api-keys/{id}/revoke",
            post(api_keys::admin_api_key_revoke),
        )
        .route(
            "/api-keys/{id}/rotate",
            post(api_keys::admin_api_key_rotate),
        )
        .route(
            "/api-keys/{id}/delete",
            post(api_keys::admin_api_key_delete),
        )
        .route("/_health/db", get(health::admin_health))
        .route("/cache/invalidate", post(cache::invalidate_cache))
        .route("/static/admin/{*path}", get(assets::serve_admin))
        .route("/static/common/{*path}", get(assets::serve_common))
        .route("/favicon.ico", get(favicon))
        .with_state(state)
        .layer(middleware::from_fn_with_state(
            response_cache,
            invalidate_admin_writes,
        ))
        .layer(middleware::from_fn(log_responses))
}

async fn favicon(State(state): State<AdminState>) -> Response {
    match state.db.load_site_settings().await {
        Ok(settings) => Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "image/svg+xml; charset=utf-8")
            .header(CACHE_CONTROL, "public, max-age=3600")
            .body(Body::from(settings.favicon_svg))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Err(err) => {
            error!(
                target = "soffio::http::admin::favicon",
                error = %err,
                "failed to load favicon from settings"
            );
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}
