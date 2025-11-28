use askama::Template;
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::application::api_keys::{ApiKeyError, ApiKeyIssued, IssueApiKeyCommand};
use crate::application::repos::SettingsRepo;
use crate::domain::api_keys::ApiScope;
use crate::infra::http::admin::selectors::API_KEYS_PANEL;
use crate::infra::http::admin::shared::{Toast, push_toasts};
use crate::presentation::admin::views as admin_views;
use crate::presentation::views::render_template_response;
use datastar::prelude::ElementPatchMode;
use std::str::FromStr;

use super::AdminState;

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyForm {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiKeyIdForm {
    pub id: String,
}

pub async fn admin_api_keys(State(state): State<AdminState>) -> Response {
    match build_page(&state, None, &[]).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn admin_api_key_create(
    State(state): State<AdminState>,
    Form(form): Form<CreateApiKeyForm>,
) -> Response {
    let scopes = match parse_scopes(&form.scopes) {
        Ok(scopes) if !scopes.is_empty() => scopes,
        _ => {
            return ApiKeyHttpError::bad_request("At least one scope is required").into_response();
        }
    };

    let expires_at = match parse_optional_time(form.expires_at.as_deref()) {
        Ok(value) => value,
        Err(err) => return err.into_response(),
    };

    let actor = "admin:api-keys";
    let issued = match state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: form.name,
            description: form.description,
            scopes,
            expires_at,
            created_by: actor.to_string(),
        })
        .await
    {
        Ok(issued) => issued,
        Err(err) => return ApiKeyHttpError::from_api(err).into_response(),
    };

    match build_stream(&state, Some(issued), &[Toast::success("API key created")]).await {
        Ok(stream) => stream,
        Err(err) => err.into_response(),
    }
}

pub async fn admin_api_key_revoke(
    State(state): State<AdminState>,
    Form(form): Form<ApiKeyIdForm>,
) -> Response {
    let id = match Uuid::parse_str(&form.id) {
        Ok(id) => id,
        Err(_) => return ApiKeyHttpError::bad_request("Invalid key id").into_response(),
    };

    if let Err(err) = state.api_keys.revoke(id).await {
        return ApiKeyHttpError::from_api(err).into_response();
    }

    match build_stream(&state, None, &[Toast::success("Key revoked")]).await {
        Ok(stream) => stream,
        Err(err) => err.into_response(),
    }
}

pub async fn admin_api_key_rotate(
    State(state): State<AdminState>,
    Form(form): Form<ApiKeyIdForm>,
) -> Response {
    let id = match Uuid::parse_str(&form.id) {
        Ok(id) => id,
        Err(_) => return ApiKeyHttpError::bad_request("Invalid key id").into_response(),
    };

    let issued = match state.api_keys.rotate(id).await {
        Ok(issued) => issued,
        Err(err) => return ApiKeyHttpError::from_api(err).into_response(),
    };

    match build_stream(&state, Some(issued), &[Toast::success("Key rotated")]).await {
        Ok(stream) => stream,
        Err(err) => err.into_response(),
    }
}

async fn build_page(
    state: &AdminState,
    issued: Option<ApiKeyIssued>,
    _toasts: &[Toast],
) -> Result<Response, ApiKeyHttpError> {
    let panel = build_panel_view(state, issued.as_ref()).await?;
    let chrome = load_chrome(state).await?;
    let template = admin_views::AdminApiKeysTemplate {
        view: admin_views::AdminLayout::new(chrome, panel),
    };
    Ok(render_template_response(template, StatusCode::OK))
}

async fn build_stream(
    state: &AdminState,
    issued: Option<ApiKeyIssued>,
    toasts: &[Toast],
) -> Result<Response, ApiKeyHttpError> {
    let panel = build_panel_view(state, issued.as_ref()).await?;
    let panel_html = render_panel_html(&panel)?;

    let mut stream = crate::application::stream::StreamBuilder::new();
    stream.push_patch(panel_html, API_KEYS_PANEL, ElementPatchMode::Replace);

    if !toasts.is_empty() {
        push_toasts(&mut stream, toasts)
            .map_err(|err| ApiKeyHttpError::service(format!("{err:?}")))?;
    }

    Ok(stream.into_response())
}

async fn build_panel_view(
    state: &AdminState,
    issued: Option<&ApiKeyIssued>,
) -> Result<admin_views::AdminApiKeyListView, ApiKeyHttpError> {
    let settings = state
        .db
        .load_site_settings()
        .await
        .map_err(ApiKeyHttpError::from_repo)?;
    let timezone = settings.timezone;

    let keys: Vec<_> = state
        .api_keys
        .list()
        .await
        .map_err(ApiKeyHttpError::from_api)?
        .into_iter()
        .map(|key| admin_views::AdminApiKeyRowView {
            id: key.id.to_string(),
            name: key.name,
            prefix: key.prefix,
            scopes: key.scopes.iter().map(|s| s.as_str().to_string()).collect(),
            created_at: admin_views::format_timestamp(key.created_at, timezone),
            last_used_at: key
                .last_used_at
                .map(|t| admin_views::format_timestamp(t, timezone)),
            expires_at: key
                .expires_at
                .map(|t| admin_views::format_timestamp(t, timezone)),
            revoked: key.revoked_at.is_some(),
            description: key.description,
            revoke_action: format!("/api-keys/{}/revoke", key.id),
            rotate_action: format!("/api-keys/{}/rotate", key.id),
        })
        .collect();

    let new_token = issued.as_ref().map(|i| i.token.clone());
    let has_keys = !keys.is_empty();

    Ok(admin_views::AdminApiKeyListView {
        heading: "API keys".to_string(),
        keys,
        create_action: "/api-keys/create".to_string(),
        available_scopes: scope_options(),
        new_token,
        has_keys,
    })
}

async fn load_chrome(state: &AdminState) -> Result<admin_views::AdminChrome, ApiKeyHttpError> {
    state
        .chrome
        .load("/api-keys")
        .await
        .map_err(ApiKeyHttpError::from_http)
}

fn render_panel_html(
    content: &admin_views::AdminApiKeyListView,
) -> Result<String, ApiKeyHttpError> {
    let template = admin_views::AdminApiKeysPanelTemplate {
        content: content.clone(),
    };
    template
        .render()
        .map_err(|err| ApiKeyHttpError::from_template(err, "admin::api_keys::panel"))
}

fn parse_scopes(values: &[String]) -> Result<Vec<ApiScope>, ApiKeyHttpError> {
    let mut scopes = Vec::new();
    for raw in values {
        match ApiScope::from_str(raw.as_str()) {
            Ok(scope) => scopes.push(scope),
            Err(_) => return Err(ApiKeyHttpError::bad_request("Invalid scope")),
        }
    }
    Ok(scopes)
}

fn parse_optional_time(value: Option<&str>) -> Result<Option<OffsetDateTime>, ApiKeyHttpError> {
    match value {
        Some(raw) => OffsetDateTime::parse(raw, &Rfc3339)
            .map(Some)
            .map_err(|err| {
                ApiKeyHttpError::bad_request_with_hint("Invalid expires_at", err.to_string())
            }),
        None => Ok(None),
    }
}

fn scope_options() -> Vec<admin_views::AdminApiScopeOption> {
    vec![
        ("content_read", "Content read"),
        ("content_write", "Content write"),
        ("tag_write", "Tags"),
        ("navigation_write", "Navigation"),
        ("upload_write", "Uploads"),
        ("settings_write", "Settings"),
        ("jobs_read", "Jobs"),
        ("audit_read", "Audit"),
    ]
    .into_iter()
    .map(|(value, label)| admin_views::AdminApiScopeOption {
        value: value.to_string(),
        label: label.to_string(),
    })
    .collect()
}

#[derive(Debug)]
struct ApiKeyHttpError(ApiErrorKind);

#[derive(Debug)]
enum ApiErrorKind {
    BadRequest(&'static str, Option<String>),
    Service(String),
}

impl ApiKeyHttpError {
    fn bad_request(message: &'static str) -> Self {
        Self(ApiErrorKind::BadRequest(message, None))
    }

    fn bad_request_with_hint(message: &'static str, hint: String) -> Self {
        Self(ApiErrorKind::BadRequest(message, Some(hint)))
    }

    fn from_api(err: ApiKeyError) -> Self {
        match err {
            ApiKeyError::InvalidScopes => Self::bad_request("invalid scopes"),
            ApiKeyError::NotFound => Self::bad_request("key not found"),
            ApiKeyError::Repo(repo) => Self(ApiErrorKind::Service(repo.to_string())),
        }
    }

    fn from_template(err: askama::Error, source: &'static str) -> Self {
        Self(ApiErrorKind::Service(format!(
            "{source} template error: {err}"
        )))
    }

    fn service(message: impl Into<String>) -> Self {
        Self(ApiErrorKind::Service(message.into()))
    }

    fn from_repo(err: crate::application::repos::RepoError) -> Self {
        Self(ApiErrorKind::Service(err.to_string()))
    }

    fn from_http(err: impl std::fmt::Debug) -> Self {
        Self::service(format!("{err:?}"))
    }
}

impl IntoResponse for ApiKeyHttpError {
    fn into_response(self) -> Response {
        match self.0 {
            ApiErrorKind::BadRequest(message, hint) => crate::application::error::HttpError::new(
                "infra::http::admin_api_keys",
                StatusCode::BAD_REQUEST,
                message,
                hint.unwrap_or_else(|| "Invalid request".to_string()),
            )
            .into_response(),
            ApiErrorKind::Service(detail) => crate::application::error::HttpError::new(
                "infra::http::admin_api_keys",
                StatusCode::INTERNAL_SERVER_ERROR,
                "API key operation failed",
                detail,
            )
            .into_response(),
        }
    }
}
