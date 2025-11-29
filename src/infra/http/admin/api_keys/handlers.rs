use askama::Template;
use axum::{
    extract::{Form, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    application::{
        api_keys::{ApiKeyIssued, IssueApiKeyCommand},
        stream::StreamBuilder,
    },
    domain::api_keys::ApiScope,
    infra::http::admin::{
        AdminState,
        pagination::CursorState,
        selectors::{API_KEYS_PANEL, SCOPE_PICKER, SCOPE_SELECTION_STORE},
        shared::{Toast, push_toasts},
    },
    presentation::{admin::views as admin_views, views::render_template_response},
};
use datastar::prelude::ElementPatchMode;
use std::str::FromStr;

use super::{
    editor::{build_new_key_view, build_scope_picker, parse_expires_in, parse_scope_state},
    errors::ApiKeyHttpError,
    forms::{ApiKeyFilters, ApiKeyIdForm, ApiKeyPanelForm, CreateApiKeyForm, ScopeToggleForm},
    panel::{build_api_key_filter, build_panel_view, render_created_panel_html, render_panel_html},
    status::parse_api_key_status,
};

pub async fn admin_api_keys(
    State(state): State<AdminState>,
    Query(filters): Query<ApiKeyFilters>,
) -> Response {
    match build_page(&state, &filters, &[]).await {
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
    let scope_values = parse_scope_state(&form.scope_state);
    let scopes = match parse_scopes(&scope_values) {
        Ok(scopes) if !scopes.is_empty() => scopes,
        _ => {
            return ApiKeyHttpError::bad_request("At least one scope is required").into_response();
        }
    };

    let expires_in = parse_expires_in(form.expires_in.as_deref());

    let actor = "admin:api-keys";
    let issued = match state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: form.name,
            description: form.description,
            scopes,
            expires_in,
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
    let scope_values = parse_scope_state(&form.scope_state);
    let scopes = match parse_scopes(&scope_values) {
        Ok(scopes) if !scopes.is_empty() => scopes,
        _ => {
            return build_error_toast_response("At least one scope is required");
        }
    };

    let expires_in = parse_expires_in(form.expires_in.as_deref());

    let actor = "admin:api-keys";
    let issued = match state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: form.name,
            description: form.description,
            scopes,
            expires_in,
            created_by: actor.to_string(),
        })
        .await
    {
        Ok(issued) => issued,
        Err(err) => {
            return build_error_toast_response(&format!("Failed to create key: {err}"));
        }
    };

    match build_created_stream(issued) {
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

pub async fn admin_api_key_delete(
    State(state): State<AdminState>,
    Form(form): Form<ApiKeyIdForm>,
) -> Response {
    let id = match Uuid::parse_str(&form.id) {
        Ok(id) => id,
        Err(_) => return ApiKeyHttpError::bad_request("Invalid key id").into_response(),
    };

    if let Err(err) = state.api_keys.delete(id).await {
        return ApiKeyHttpError::from_api(err).into_response();
    }

    let filters = ApiKeyFilters {
        status: form.status_filter,
        search: form.filter_search,
        scope: form.filter_scope,
        cursor: form.cursor,
        trail: form.trail,
    };

    match build_stream(&state, None, &filters, &[Toast::success("Key deleted")]).await {
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

fn render_scope_picker_response(selected_scopes: &[String]) -> Response {
    let picker = build_scope_picker(selected_scopes);

    let picker_template = admin_views::AdminApiKeyScopePickerTemplate {
        picker: picker.clone(),
    };
    let picker_html = match picker_template.render() {
        Ok(html) => html,
        Err(err) => {
            return ApiKeyHttpError::from_template(err, "admin::api_keys::scope_picker")
                .into_response();
        }
    };

    let store_template = admin_views::AdminApiKeyScopeSelectionStoreTemplate { picker };
    let store_html = match store_template.render() {
        Ok(html) => html,
        Err(err) => {
            return ApiKeyHttpError::from_template(err, "admin::api_keys::scope_selection_store")
                .into_response();
        }
    };

    let mut stream = StreamBuilder::new();
    stream.push_patch(picker_html, SCOPE_PICKER, ElementPatchMode::Replace);
    stream.push_patch(store_html, SCOPE_SELECTION_STORE, ElementPatchMode::Replace);
    stream.into_response()
}

async fn build_page(
    state: &AdminState,
    filters: &ApiKeyFilters,
    _toasts: &[Toast],
) -> Result<Response, ApiKeyHttpError> {
    let status_filter =
        parse_api_key_status(filters.status.as_deref()).map_err(ApiKeyHttpError::from_http)?;

    let query_filter = build_api_key_filter(filters.scope.as_deref(), filters.search.as_deref());

    let cursor_state = CursorState::new(filters.cursor.clone(), filters.trail.clone());

    let panel = build_panel_view(state, status_filter, &query_filter, &cursor_state, None).await?;
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
    let view = build_new_key_view(issued.map(|i| i.token));

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
    let status_filter =
        parse_api_key_status(filters.status.as_deref()).map_err(ApiKeyHttpError::from_http)?;

    let query_filter = build_api_key_filter(filters.scope.as_deref(), filters.search.as_deref());

    let cursor_state = CursorState::new(filters.cursor.clone(), filters.trail.clone());

    let panel = build_panel_view(
        state,
        status_filter,
        &query_filter,
        &cursor_state,
        issued.as_ref(),
    )
    .await?;
    let panel_html = render_panel_html(&panel)?;

    let mut stream = StreamBuilder::new();
    stream.push_patch(panel_html, API_KEYS_PANEL, ElementPatchMode::Replace);

    if !toasts.is_empty() {
        push_toasts(&mut stream, toasts)
            .map_err(|err| ApiKeyHttpError::service(format!("{err:?}")))?;
    }

    Ok(stream.into_response())
}

fn build_error_toast_response(message: &str) -> Response {
    let mut stream = StreamBuilder::new();
    if let Err(err) = push_toasts(&mut stream, &[Toast::error(message)]) {
        return ApiKeyHttpError::service(format!("{err:?}")).into_response();
    }
    stream.into_response()
}

fn build_created_stream(issued: ApiKeyIssued) -> Result<Response, ApiKeyHttpError> {
    let created_view = admin_views::AdminApiKeyCreatedView {
        heading: "New key".to_string(),
        token: issued.token,
        back_href: "/api-keys".to_string(),
        copy_toast_action: "/toasts".to_string(),
    };
    let panel_html = render_created_panel_html(&created_view)?;

    let mut stream = StreamBuilder::new();
    stream.push_patch(panel_html, API_KEYS_PANEL, ElementPatchMode::Replace);
    push_toasts(&mut stream, &[Toast::success("API key created")])
        .map_err(|err| ApiKeyHttpError::service(format!("{err:?}")))?;

    Ok(stream.into_response())
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

async fn load_chrome(state: &AdminState) -> Result<admin_views::AdminChrome, ApiKeyHttpError> {
    state
        .chrome
        .load("/api-keys")
        .await
        .map_err(ApiKeyHttpError::from_http)
}
