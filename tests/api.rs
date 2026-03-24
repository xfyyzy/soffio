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

#[path = "api/rate_limit.rs"]
mod rate_limit;

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
