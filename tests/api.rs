use axum::body::Body;
use axum::extract::{Extension, Json, Path, Query, State};
use axum::http::{Request, StatusCode};
use axum::middleware;
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use std::sync::Arc;

use sqlx::PgPool;
use time::OffsetDateTime;

use soffio::application::api_keys::IssueApiKeyCommand;
use soffio::domain::api_keys::ApiScope;
use soffio::domain::entities::UploadRecord;
use soffio::infra::http::api::handlers;
use soffio::infra::http::api::models::*;
use soffio::infra::http::api::state::ApiState;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "support/mod.rs"]
mod support;

use support::api_harness::{build_state, response_json, string_field, uuid_field};

#[sqlx::test(migrations = "./migrations")]
async fn api_rate_limit_uses_route_template(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state
        .api_keys
        .authenticate(&token)
        .await
        .expect("authenticate token");

    let state = ApiState {
        rate_limiter: Arc::new(soffio::infra::http::api::rate_limit::ApiRateLimiter::new(
            std::time::Duration::from_secs(60),
            1,
        )),
        ..state
    };

    let app = Router::new()
        .route("/api/v1/posts/{id}", get(|| async { StatusCode::OK }))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state,
            soffio::infra::http::api::middleware::api_rate_limit,
        ));

    let first_path = format!("/api/v1/posts/{}", Uuid::new_v4());
    let mut first_request = Request::builder()
        .method("GET")
        .uri(first_path)
        .body(Body::empty())
        .expect("build first request");
    first_request.extensions_mut().insert(principal.clone());

    let first_response = app
        .clone()
        .oneshot(first_request)
        .await
        .expect("send first request");
    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(
        first_response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|value| value.to_str().ok()),
        Some("0")
    );

    let second_path = format!("/api/v1/posts/{}", Uuid::new_v4());
    let mut second_request = Request::builder()
        .method("GET")
        .uri(second_path)
        .body(Body::empty())
        .expect("build second request");
    second_request.extensions_mut().insert(principal);

    let second_response = app
        .oneshot(second_request)
        .await
        .expect("send second request");
    assert_eq!(second_response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[path = "api/posts.rs"]
mod posts;

#[path = "api/pages.rs"]
mod pages;

#[path = "api/tags.rs"]
mod tags;

#[path = "api/navigation.rs"]
mod navigation;

#[path = "api/uploads.rs"]
mod uploads;

#[path = "api/settings.rs"]
mod settings;

#[path = "api/jobs.rs"]
mod jobs;

#[path = "api/audit.rs"]
mod audit;

#[path = "api/api_keys.rs"]
mod api_keys;
