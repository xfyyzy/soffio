use askama::Template;
use axum::{
    extract::{Form, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::api_keys::{ApiKeyError, ApiKeyIssued, IssueApiKeyCommand};
use crate::application::pagination::ApiKeyCursor;
use crate::application::repos::{
    ApiKeyPageRequest, ApiKeyQueryFilter, ApiKeyStatusFilter, SettingsRepo,
};
use crate::domain::api_keys::ApiScope;
use crate::infra::http::admin::pagination::{self, CursorState};
use crate::infra::http::admin::selectors::API_KEYS_PANEL;
use crate::infra::http::admin::shared::{Toast, push_toasts};
use crate::presentation::admin::views as admin_views;
use crate::presentation::views::render_template_response;
use datastar::prelude::ElementPatchMode;
use std::str::FromStr;

use super::AdminState;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ApiKeyFilters {
    pub status: Option<String>,
    pub search: Option<String>,
    pub scope: Option<String>,
    pub cursor: Option<String>,
    pub trail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyForm {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub expires_in: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiKeyIdForm {
    pub id: String,
    pub status_filter: Option<String>,
    pub filter_search: Option<String>,
    pub filter_scope: Option<String>,
    pub cursor: Option<String>,
    pub trail: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ApiKeyPanelForm {
    pub status: Option<String>,
    pub search: Option<String>,
    pub scope: Option<String>,
    pub cursor: Option<String>,
    pub trail: Option<String>,
    pub clear: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScopeToggleForm {
    pub scope_id: String,
    pub scope_state: Option<String>,
}

pub async fn admin_api_keys(
    State(state): State<AdminState>,
    Query(filters): Query<ApiKeyFilters>,
) -> Response {
    match build_page(&state, None, &filters, &[]).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

pub async fn admin_api_keys_panel(
    State(state): State<AdminState>,
    Form(form): Form<ApiKeyPanelForm>,
) -> Response {
    let filters = if form.clear.is_some() {
        ApiKeyFilters::default()
    } else {
        ApiKeyFilters {
            status: form.status,
            search: form.search,
            scope: form.scope,
            cursor: form.cursor,
            trail: form.trail,
        }
    };

    match build_stream(&state, None, &filters, &[]).await {
        Ok(stream) => stream,
        Err(err) => err.into_response(),
    }
}

pub async fn admin_api_key_new(State(state): State<AdminState>) -> Response {
    match build_new_page(&state, None, StatusCode::OK).await {
        Ok(resp) => resp,
        Err(err) => err.into_response(),
    }
}

pub async fn admin_api_key_new_submit(
    State(state): State<AdminState>,
    Form(form): Form<CreateApiKeyForm>,
) -> Response {
    let scopes = match parse_scopes(&form.scopes) {
        Ok(scopes) if !scopes.is_empty() => scopes,
        _ => {
            return ApiKeyHttpError::bad_request("At least one scope is required").into_response();
        }
    };

    let expires_at = parse_expires_in(form.expires_in.as_deref());

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

    match build_new_page(&state, Some(issued), StatusCode::CREATED).await {
        Ok(resp) => resp,
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

    let expires_at = parse_expires_in(form.expires_in.as_deref());

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

    match build_stream(
        &state,
        Some(issued),
        &ApiKeyFilters::default(),
        &[Toast::success("API key created")],
    )
    .await
    {
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

    let filters = ApiKeyFilters {
        status: form.status_filter,
        search: form.filter_search,
        scope: form.filter_scope,
        cursor: form.cursor,
        trail: form.trail,
    };

    match build_stream(&state, None, &filters, &[Toast::success("Key revoked")]).await {
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

    let filters = ApiKeyFilters {
        status: form.status_filter,
        search: form.filter_search,
        scope: form.filter_scope,
        cursor: form.cursor,
        trail: form.trail,
    };

    match build_stream(
        &state,
        Some(issued),
        &filters,
        &[Toast::success("Key rotated")],
    )
    .await
    {
        Ok(stream) => stream,
        Err(err) => err.into_response(),
    }
}

pub async fn admin_api_key_scopes_toggle(
    State(_state): State<AdminState>,
    Form(form): Form<ScopeToggleForm>,
) -> Response {
    let mut selected = parse_scope_state(&form.scope_state);

    if let Some(index) = selected.iter().position(|s| *s == form.scope_id) {
        selected.remove(index);
    } else {
        selected.push(form.scope_id);
    }

    render_scope_picker_response(&selected)
}

fn parse_scope_state(state: &Option<String>) -> Vec<String> {
    state
        .as_deref()
        .map(|raw| {
            raw.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn render_scope_picker_response(selected_scopes: &[String]) -> Response {
    use crate::infra::http::admin::selectors::SCOPE_PICKER;

    let picker = build_scope_picker(selected_scopes);
    let template = admin_views::AdminApiKeyScopePickerTemplate { picker };

    let html = match template.render() {
        Ok(html) => html,
        Err(err) => {
            return ApiKeyHttpError::from_template(err, "admin::api_keys::scope_picker")
                .into_response();
        }
    };

    let mut stream = crate::application::stream::StreamBuilder::new();
    stream.push_patch(html, SCOPE_PICKER, ElementPatchMode::Replace);
    stream.into_response()
}

async fn build_page(
    state: &AdminState,
    issued: Option<ApiKeyIssued>,
    filters: &ApiKeyFilters,
    _toasts: &[Toast],
) -> Result<Response, ApiKeyHttpError> {
    let panel = build_panel_view(state, filters, issued.as_ref()).await?;
    let chrome = load_chrome(state).await?;
    let template = admin_views::AdminApiKeysTemplate {
        view: admin_views::AdminLayout::new(chrome, panel),
    };
    Ok(render_template_response(template, StatusCode::OK))
}

async fn build_new_page(
    state: &AdminState,
    issued: Option<ApiKeyIssued>,
    status: StatusCode,
) -> Result<Response, ApiKeyHttpError> {
    let chrome = load_chrome(state).await?;
    let view = admin_views::AdminApiKeyNewView {
        heading: "Create API key".to_string(),
        form_action: "/api-keys/new".to_string(),
        name: None,
        description: None,
        expires_in_options: expires_in_options(None),
        scope_picker: build_scope_picker(&[]),
        new_token: issued.map(|i| i.token),
    };

    let layout = admin_views::AdminLayout::new(chrome, view);
    let template = admin_views::AdminApiKeyNewTemplate { view: layout };
    Ok(render_template_response(template, status))
}

async fn build_stream(
    state: &AdminState,
    issued: Option<ApiKeyIssued>,
    filters: &ApiKeyFilters,
    toasts: &[Toast],
) -> Result<Response, ApiKeyHttpError> {
    let panel = build_panel_view(state, filters, issued.as_ref()).await?;
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
    filters: &ApiKeyFilters,
    issued: Option<&ApiKeyIssued>,
) -> Result<admin_views::AdminApiKeyListView, ApiKeyHttpError> {
    let settings = state
        .db
        .load_site_settings()
        .await
        .map_err(ApiKeyHttpError::from_repo)?;
    let timezone = settings.timezone;

    let query_filter = ApiKeyQueryFilter {
        status: match filters.status.as_deref() {
            Some("active") => Some(ApiKeyStatusFilter::Active),
            Some("revoked") => Some(ApiKeyStatusFilter::Revoked),
            _ => None,
        },
        scope: filters
            .scope
            .as_deref()
            .and_then(|s| ApiScope::from_str(s).ok()),
        search: filters.search.clone(),
    };

    let cursor_state = CursorState::new(filters.cursor.clone(), filters.trail.clone());
    let cursor = cursor_state
        .decode_with(ApiKeyCursor::decode, "admin_api_keys")
        .map_err(ApiKeyHttpError::from_http)?;

    let page_req = ApiKeyPageRequest {
        limit: settings.admin_page_size as u32,
        cursor,
    };

    let page = state
        .api_keys
        .list_page(&query_filter, page_req)
        .await
        .map_err(ApiKeyHttpError::from_api)?;

    let keys: Vec<_> = page
        .items
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

    let total_count = page.total;
    let revoked_count = page.revoked;
    let new_token = issued.as_ref().map(|i| i.token.clone());

    let mut previous_history = cursor_state.clone_history();
    let previous_token = previous_history.pop();
    let previous_page_state = previous_token.map(|token| {
        let prev_cursor_value = pagination::decode_cursor_token(&token);
        let prev_trail = pagination::join_cursor_history(&previous_history);
        admin_views::AdminApiKeyPaginationState {
            cursor: prev_cursor_value,
            trail: prev_trail,
        }
    });

    let next_page_state = page.next_cursor.map(|c| {
        let mut next_history = cursor_state.clone_history();
        next_history.push(pagination::encode_cursor_token(
            cursor_state.current_token_ref(),
        ));
        let next_trail = pagination::join_cursor_history(&next_history);
        admin_views::AdminApiKeyPaginationState {
            cursor: Some(c.encode()),
            trail: next_trail,
        }
    });

    // scope filter options from aggregated counts
    let mut scope_filter_options: Vec<admin_views::AdminPostTagOption> = page
        .scope_counts
        .iter()
        .map(|(scope, count)| admin_views::AdminPostTagOption {
            slug: scope.as_str().to_string(),
            name: scope.as_str().to_string(),
            count: *count,
        })
        .collect();
    scope_filter_options.sort_by(|a, b| a.name.cmp(&b.name));

    let filters_view = build_status_filters(filters.status.as_deref(), total_count, revoked_count);
    let status_filter_active = filters_view
        .iter()
        .find(|f| f.is_active)
        .and_then(|f| f.status_key.clone());

    let status_filters = filters_view;

    Ok(admin_views::AdminApiKeyListView {
        heading: "API keys".to_string(),
        keys,
        create_action: "/api-keys/create".to_string(),
        new_key_href: "/api-keys/new".to_string(),
        panel_action: "/api-keys/panel".to_string(),
        filters: status_filters,
        active_status_key: status_filter_active,
        filter_search: filters.search.clone(),
        filter_scope: filters.scope.clone(),
        filter_tag: filters.scope.clone(),
        filter_month: None,
        tag_filter_enabled: true,
        month_filter_enabled: false,
        tag_filter_label: "Scope".to_string(),
        tag_filter_all_label: "All scopes".to_string(),
        tag_filter_field: "scope".to_string(),
        tag_options: scope_filter_options,
        month_options: Vec::new(),
        cursor_param: filters.cursor.clone(),
        trail: filters.trail.clone(),
        previous_page_state,
        next_page_state,
        available_scopes: scope_options(),
        new_token,
    })
}

fn build_status_filters(
    active: Option<&str>,
    total_count: u64,
    revoked_count: u64,
) -> Vec<admin_views::AdminApiKeyStatusFilterView> {
    let active_count = total_count.saturating_sub(revoked_count);
    vec![
        admin_views::AdminApiKeyStatusFilterView {
            status_key: None,
            label: "All".to_string(),
            count: total_count,
            is_active: active.is_none(),
        },
        admin_views::AdminApiKeyStatusFilterView {
            status_key: Some("active".to_string()),
            label: "Active".to_string(),
            count: active_count,
            is_active: active == Some("active"),
        },
        admin_views::AdminApiKeyStatusFilterView {
            status_key: Some("revoked".to_string()),
            label: "Revoked".to_string(),
            count: revoked_count,
            is_active: active == Some("revoked"),
        },
    ]
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

fn expires_in_options(selected: Option<&str>) -> Vec<admin_views::AdminApiKeyExpiresInOption> {
    vec![
        ("", "Never expires"),
        ("30d", "30 days"),
        ("90d", "90 days"),
        ("180d", "180 days"),
        ("1y", "1 year"),
    ]
    .into_iter()
    .map(|(value, label)| admin_views::AdminApiKeyExpiresInOption {
        value: value.to_string(),
        label: label.to_string(),
        selected: selected == Some(value) || (selected.is_none() && value.is_empty()),
    })
    .collect()
}

fn build_scope_picker(selected_scopes: &[String]) -> admin_views::AdminApiKeyScopePickerView {
    let all_scopes = scope_options();
    let selected: Vec<admin_views::AdminApiScopeOption> = all_scopes
        .iter()
        .filter(|s| selected_scopes.contains(&s.value))
        .cloned()
        .collect();
    let available: Vec<admin_views::AdminApiScopeOption> = all_scopes
        .iter()
        .filter(|s| !selected_scopes.contains(&s.value))
        .cloned()
        .collect();
    admin_views::AdminApiKeyScopePickerView {
        toggle_action: "/api-keys/new/scopes/toggle".to_string(),
        selected,
        available,
        selected_values: selected_scopes.to_vec(),
    }
}

fn parse_expires_in(value: Option<&str>) -> Option<OffsetDateTime> {
    use time::Duration;
    let now = OffsetDateTime::now_utc();
    match value {
        None | Some("") => None, // Never expires
        Some("30d") => Some(now + Duration::days(30)),
        Some("90d") => Some(now + Duration::days(90)),
        Some("180d") => Some(now + Duration::days(180)),
        Some("1y") => Some(now + Duration::days(365)),
        _ => None, // Unknown value, treat as never expires
    }
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
