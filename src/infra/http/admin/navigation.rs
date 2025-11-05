use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use url::form_urlencoded::Serializer;
use uuid::Uuid;

use crate::{
    application::{
        admin::navigation::{
            AdminNavigationError, CreateNavigationItemCommand, NavigationStatusCounts,
            UpdateNavigationItemCommand,
        },
        error::HttpError,
        pagination::{NavigationCursor, PageRequest},
        repos::{NavigationQueryFilter, SettingsRepo},
    },
    domain::{entities::NavigationItemRecord, types::NavigationDestinationType},
    infra::http::admin::{
        AdminState,
        pagination::{self, CursorState},
        selectors::{NAVIGATION_PANEL, PANEL},
        shared::{
            EditorSuccessRender, Toast, blank_to_none_opt, datastar_replace, push_toasts,
            stream_editor_success, template_render_http_error,
        },
    },
    presentation::{admin::views as admin_views, views::render_template_response},
};

#[derive(Debug, Deserialize)]
pub(super) struct AdminNavigationQuery {
    status: Option<String>,
    cursor: Option<String>,
    trail: Option<String>,
    search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminNavigationPanelForm {
    status: Option<String>,
    search: Option<String>,
    cursor: Option<String>,
    trail: Option<String>,
    clear: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminNavigationForm {
    label: String,
    destination_type: String,
    destination_page_id: Option<String>,
    destination_url: Option<String>,
    sort_order: i32,
    visible: Option<String>,
    open_in_new_tab: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminNavigationDeleteForm {
    status: Option<String>,
    search: Option<String>,
    cursor: Option<String>,
    trail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AdminNavigationVisibilityForm {
    status: Option<String>,
    search: Option<String>,
    cursor: Option<String>,
    trail: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NavigationListStatus {
    All,
    Visible,
    Hidden,
}

pub(super) async fn admin_navigation(
    State(state): State<AdminState>,
    Query(query): Query<AdminNavigationQuery>,
) -> Response {
    let chrome = match state.chrome.load("/navigation").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(query.cursor.clone(), query.trail.clone());

    let status = match parse_navigation_status(query.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let cursor =
        match cursor_state.decode_with(NavigationCursor::decode, "infra::http::admin_navigation") {
            Ok(cursor) => cursor,
            Err(err) => return err.into_response(),
        };

    let filter = build_navigation_filter(query.search.as_deref());

    let mut content = match build_navigation_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation", err).into_response();
        }
    };

    apply_navigation_pagination_links(&mut content, &cursor_state);

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(
        admin_views::AdminNavigationTemplate { view },
        StatusCode::OK,
    )
}

pub(super) async fn admin_navigation_panel(
    State(state): State<AdminState>,
    Form(form): Form<AdminNavigationPanelForm>,
) -> Response {
    let status = match parse_navigation_status(form.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let cursor = match cursor_state.decode_with(
        NavigationCursor::decode,
        "infra::http::admin_navigation_panel",
    ) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let search = if form.clear.is_some() {
        None
    } else {
        form.search.as_deref()
    };
    let filter = build_navigation_filter(search);

    let mut content = match build_navigation_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_panel", err)
                .into_response();
        }
    };

    apply_navigation_pagination_links(&mut content, &cursor_state);

    let panel_html =
        match render_navigation_panel_html(&content, "infra::http::admin_navigation_panel") {
            Ok(html) => html,
            Err(err) => return err.into_response(),
        };

    datastar_replace(NAVIGATION_PANEL, panel_html).into_response()
}

pub(super) async fn admin_navigation_new(State(state): State<AdminState>) -> Response {
    let chrome = match state.chrome.load("/navigation").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let content = match build_navigation_editor_view(&state, None, None).await {
        Ok(view) => view,
        Err(err) => return err.into_response(),
    };

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(
        admin_views::AdminNavigationEditTemplate { view },
        StatusCode::OK,
    )
}

pub(super) async fn admin_navigation_edit(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
) -> Response {
    let chrome = match state.chrome.load("/navigation").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let item = match state.navigation.find_by_id(id).await {
        Ok(Some(item)) => item,
        Ok(None) => return Redirect::to("/navigation").into_response(),
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_edit", err)
                .into_response();
        }
    };

    let content = match build_navigation_editor_view(&state, Some(&item), None).await {
        Ok(view) => view,
        Err(err) => return err.into_response(),
    };

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(
        admin_views::AdminNavigationEditTemplate { view },
        StatusCode::OK,
    )
}

pub(super) async fn admin_navigation_create(
    State(state): State<AdminState>,
    Form(form): Form<AdminNavigationForm>,
) -> Response {
    let destination_type = match parse_navigation_type(&form.destination_type) {
        Ok(value) => value,
        Err(err) => return err.into_response(),
    };

    let destination_page_id = parse_optional_uuid(form.destination_page_id.as_deref());

    let command = CreateNavigationItemCommand {
        label: form.label.trim().to_string(),
        destination_type,
        destination_page_id,
        destination_url: blank_to_none_opt(form.destination_url.clone()),
        sort_order: form.sort_order,
        visible: form.visible.is_some(),
        open_in_new_tab: form.open_in_new_tab.is_some(),
    };

    let actor = "admin";

    match state.navigation.create_item(actor, command).await {
        Ok(item) => {
            let list_filter = NavigationQueryFilter::default();
            handle_editor_success(
                &state,
                NavigationEditorSuccess {
                    item: Some(&item),
                    form: None,
                    toasts: &[Toast::success(format!(
                        "Created navigation item \"{}\"",
                        item.label
                    ))],
                    template_source: "infra::http::admin_navigation_create",
                    replace_id: Some(item.id),
                    panel_status: NavigationListStatus::All,
                    panel_filter: &list_filter,
                },
            )
            .await
        }
        Err(err) => {
            handle_editor_failure(
                &state,
                None,
                Some(&form),
                &[Toast::error(format!(
                    "Failed to create navigation item: {}",
                    err
                ))],
                "infra::http::admin_navigation_create",
            )
            .await
        }
    }
}

pub(super) async fn admin_navigation_update(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminNavigationForm>,
) -> Response {
    let destination_type = match parse_navigation_type(&form.destination_type) {
        Ok(value) => value,
        Err(err) => return err.into_response(),
    };

    let destination_page_id = parse_optional_uuid(form.destination_page_id.as_deref());

    let command = UpdateNavigationItemCommand {
        id,
        label: form.label.trim().to_string(),
        destination_type,
        destination_page_id,
        destination_url: blank_to_none_opt(form.destination_url.clone()),
        sort_order: form.sort_order,
        visible: form.visible.is_some(),
        open_in_new_tab: form.open_in_new_tab.is_some(),
    };

    let actor = "admin";

    match state.navigation.update_item(actor, command).await {
        Ok(item) => {
            let list_filter = NavigationQueryFilter::default();
            handle_editor_success(
                &state,
                NavigationEditorSuccess {
                    item: Some(&item),
                    form: None,
                    toasts: &[Toast::success(format!(
                        "Saved navigation item \"{}\"",
                        item.label
                    ))],
                    template_source: "infra::http::admin_navigation_update",
                    replace_id: Some(item.id),
                    panel_status: NavigationListStatus::All,
                    panel_filter: &list_filter,
                },
            )
            .await
        }
        Err(err) => {
            let existing = match state.navigation.find_by_id(id).await {
                Ok(record) => record,
                Err(repo_err) => {
                    return admin_navigation_error(
                        "infra::http::admin_navigation_update",
                        repo_err,
                    )
                    .into_response();
                }
            };

            handle_editor_failure(
                &state,
                existing.as_ref(),
                Some(&form),
                &[Toast::error(format!(
                    "Failed to save navigation item: {}",
                    err
                ))],
                "infra::http::admin_navigation_update",
            )
            .await
        }
    }
}

pub(super) async fn admin_navigation_destination_preview(
    State(state): State<AdminState>,
    Form(form): Form<AdminNavigationForm>,
) -> Response {
    handle_editor_preview(&state, None, form, "infra::http::admin_navigation_preview").await
}

pub(super) async fn admin_navigation_destination_preview_for_item(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminNavigationForm>,
) -> Response {
    handle_editor_preview(
        &state,
        Some(id),
        form,
        "infra::http::admin_navigation_preview",
    )
    .await
}

pub(super) async fn admin_navigation_toggle_visibility(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminNavigationVisibilityForm>,
) -> Response {
    let status = match parse_navigation_status(form.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let cursor = match cursor_state.decode_with(
        NavigationCursor::decode,
        "infra::http::admin_navigation_toggle_visibility",
    ) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = build_navigation_filter(form.search.as_deref());

    let item = match state.navigation.find_by_id(id).await {
        Ok(Some(item)) => item,
        Ok(None) => {
            return render_navigation_error_panel(
                &state,
                status,
                &filter,
                cursor,
                &cursor_state,
                "Navigation item not found",
            )
            .await;
        }
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_toggle_visibility", err)
                .into_response();
        }
    };

    let new_visibility = !item.visible;
    let (destination_page_id, destination_url) = match item.destination_type {
        NavigationDestinationType::Internal => {
            if let Some(page_id) = item.destination_page_id {
                (Some(page_id), None)
            } else {
                return render_navigation_error_panel(
                    &state,
                    status,
                    &filter,
                    cursor,
                    &cursor_state,
                    format!(
                        "Navigation item \"{}\" is missing an internal destination",
                        item.label
                    ),
                )
                .await;
            }
        }
        NavigationDestinationType::External => (None, item.destination_url.clone()),
    };

    let command = UpdateNavigationItemCommand {
        id,
        label: item.label.clone(),
        destination_type: item.destination_type,
        destination_page_id,
        destination_url,
        sort_order: item.sort_order,
        visible: new_visibility,
        open_in_new_tab: item.open_in_new_tab,
    };

    let actor = "admin";

    let result = state.navigation.update_item(actor, command).await;

    let mut content = match build_navigation_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_toggle_visibility", err)
                .into_response();
        }
    };

    apply_navigation_pagination_links(&mut content, &cursor_state);

    let panel_html = match render_navigation_panel_html(
        &content,
        "infra::http::admin_navigation_toggle_visibility",
    ) {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    let mut stream = datastar_replace(NAVIGATION_PANEL, panel_html);

    match result {
        Ok(updated) => {
            let message = if updated.visible {
                format!("Marked navigation item \"{}\" as visible", updated.label)
            } else {
                format!("Marked navigation item \"{}\" as hidden", updated.label)
            };
            if let Err(err) = push_toasts(&mut stream, &[Toast::success(message)]) {
                return err.into_response();
            }
        }
        Err(err) => {
            if let Err(push_err) = push_toasts(
                &mut stream,
                &[Toast::error(format!(
                    "Failed to toggle navigation item visibility: {}",
                    err
                ))],
            ) {
                return push_err.into_response();
            }
        }
    }

    stream.into_response()
}

pub(super) async fn admin_navigation_delete(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminNavigationDeleteForm>,
) -> Response {
    let status = match parse_navigation_status(form.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let cursor = match cursor_state.decode_with(
        NavigationCursor::decode,
        "infra::http::admin_navigation_delete",
    ) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = build_navigation_filter(form.search.as_deref());

    let item = match state.navigation.find_by_id(id).await {
        Ok(Some(item)) => item,
        Ok(None) => {
            let panel_html = match build_navigation_panel_html(
                &state,
                status,
                &filter,
                "infra::http::admin_navigation_delete",
                "infra::http::admin_navigation_delete",
            )
            .await
            {
                Ok(html) => html,
                Err(err) => return err.into_response(),
            };

            let mut stream = datastar_replace(NAVIGATION_PANEL, panel_html);
            if let Err(err) = push_toasts(
                &mut stream,
                &[Toast::error("Navigation item not found".to_string())],
            ) {
                return err.into_response();
            }
            return stream.into_response();
        }
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_delete", err)
                .into_response();
        }
    };

    if let Err(err) = state.navigation.delete_item("admin", id).await {
        return admin_navigation_error("infra::http::admin_navigation_delete", err).into_response();
    }

    let mut content = match build_navigation_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_delete", err)
                .into_response();
        }
    };

    apply_navigation_pagination_links(&mut content, &cursor_state);

    let panel_html =
        match render_navigation_panel_html(&content, "infra::http::admin_navigation_delete") {
            Ok(html) => html,
            Err(err) => return err.into_response(),
        };

    let mut stream = datastar_replace(NAVIGATION_PANEL, panel_html);
    if let Err(err) = push_toasts(
        &mut stream,
        &[Toast::success(format!(
            "Deleted navigation item \"{}\"",
            item.label
        ))],
    ) {
        return err.into_response();
    }

    stream.into_response()
}

struct NavigationEditorSuccess<'a> {
    item: Option<&'a NavigationItemRecord>,
    form: Option<&'a AdminNavigationForm>,
    toasts: &'a [Toast],
    template_source: &'static str,
    replace_id: Option<Uuid>,
    panel_status: NavigationListStatus,
    panel_filter: &'a NavigationQueryFilter,
}

async fn handle_editor_success(
    state: &AdminState,
    params: NavigationEditorSuccess<'_>,
) -> Response {
    let NavigationEditorSuccess {
        item,
        form,
        toasts,
        template_source,
        replace_id,
        panel_status,
        panel_filter,
    } = params;

    let content = match build_navigation_editor_view(state, item, form).await {
        Ok(view) => view,
        Err(err) => return err.into_response(),
    };

    let editor_html = match render_navigation_editor_panel(&content, template_source) {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    let panel_html = match build_navigation_panel_html(
        state,
        panel_status,
        panel_filter,
        template_source,
        template_source,
    )
    .await
    {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    stream_editor_success(EditorSuccessRender {
        editor_html,
        panel_html,
        panel_selector: NAVIGATION_PANEL,
        toasts,
        history_path: replace_id.map(|id| format!("/navigation/{}/edit", id)),
    })
}

async fn handle_editor_failure(
    state: &AdminState,
    item: Option<&NavigationItemRecord>,
    form: Option<&AdminNavigationForm>,
    toasts: &[Toast],
    template_source: &'static str,
) -> Response {
    let content = match build_navigation_editor_view(state, item, form).await {
        Ok(view) => view,
        Err(err) => return err.into_response(),
    };

    let panel_html = match render_navigation_editor_panel(&content, template_source) {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    let mut stream = datastar_replace(PANEL, panel_html);
    if let Err(err) = push_toasts(&mut stream, toasts) {
        return err.into_response();
    }

    stream.into_response()
}

async fn handle_editor_preview(
    state: &AdminState,
    id: Option<Uuid>,
    form: AdminNavigationForm,
    template_source: &'static str,
) -> Response {
    let record = match id {
        Some(id) => match state.navigation.find_by_id(id).await {
            Ok(record) => record,
            Err(err) => {
                return admin_navigation_error("infra::http::admin_navigation_preview", err)
                    .into_response();
            }
        },
        None => None,
    };

    let content = match build_navigation_editor_view(state, record.as_ref(), Some(&form)).await {
        Ok(view) => view,
        Err(err) => return err.into_response(),
    };

    let panel_html = match render_navigation_editor_panel(&content, template_source) {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    datastar_replace(PANEL, panel_html).into_response()
}

fn apply_navigation_pagination_links(
    content: &mut admin_views::AdminNavigationListView,
    cursor_state: &CursorState,
) {
    content.cursor_param = cursor_state.current_token();
    content.trail = pagination::join_cursor_history(cursor_state.history_tokens());

    let mut previous_history = cursor_state.clone_history();
    let previous_token = previous_history.pop();

    content.previous_page_state = previous_token.map(|token| {
        let previous_cursor_value = pagination::decode_cursor_token(&token);
        let previous_trail = pagination::join_cursor_history(&previous_history);
        admin_views::AdminPostPaginationState {
            cursor: previous_cursor_value,
            trail: previous_trail,
        }
    });

    if let Some(next_cursor) = content.next_cursor.clone() {
        let mut next_history = cursor_state.clone_history();
        next_history.push(pagination::encode_cursor_token(
            cursor_state.current_token_ref(),
        ));
        let next_trail = pagination::join_cursor_history(&next_history);
        content.next_page_state = Some(admin_views::AdminPostPaginationState {
            cursor: Some(next_cursor),
            trail: next_trail,
        });
    } else {
        content.next_page_state = None;
    }
}

async fn build_navigation_list_view(
    state: &AdminState,
    status: NavigationListStatus,
    filter: &NavigationQueryFilter,
    cursor: Option<NavigationCursor>,
) -> Result<admin_views::AdminNavigationListView, AdminNavigationError> {
    let settings = state.db.load_site_settings().await?;
    let admin_page_size = settings.admin_page_size.clamp(1, 100).max(1) as u32;
    let public_site_url = normalize_public_site_url(&settings.public_site_url);

    let counts_future = state.navigation.status_counts(filter);
    let list_future = state.navigation.list(
        status.visibility(),
        filter,
        PageRequest::new(admin_page_size, cursor),
    );

    let (counts, page) = tokio::try_join!(counts_future, list_future)?;

    let filters = status_filters(&counts, status);

    let items = page
        .items
        .into_iter()
        .map(|item| map_navigation_row(&item, &public_site_url))
        .collect();

    let mut serializer = Serializer::new(String::new());
    if let Some(search) = filter.search.as_ref() {
        serializer.append_pair("search", search);
    }
    let filter_query = serializer.finish();

    Ok(admin_views::AdminNavigationListView {
        heading: "Navigation".to_string(),
        filters,
        items,
        filter_search: filter.search.clone(),
        filter_tag: None,
        filter_month: None,
        filter_query,
        next_cursor: page.next_cursor,
        cursor_param: None,
        trail: None,
        previous_page_state: None,
        next_page_state: None,
        tag_options: Vec::new(),
        month_options: Vec::new(),
        tag_filter_enabled: false,
        month_filter_enabled: false,
        panel_action: "/navigation/panel".to_string(),
        active_status_key: status_key(status).map(|s| s.to_string()),
        new_navigation_href: "/navigation/new".to_string(),
        tag_filter_label: "Tag".to_string(),
        tag_filter_all_label: "All tags".to_string(),
        tag_filter_field: "tag".to_string(),
    })
}

fn render_navigation_panel_html(
    content: &admin_views::AdminNavigationListView,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let template = admin_views::AdminNavigationPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}

async fn build_navigation_panel_html(
    state: &AdminState,
    status: NavigationListStatus,
    filter: &NavigationQueryFilter,
    error_source: &'static str,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let content = build_navigation_list_view(state, status, filter, None)
        .await
        .map_err(|err| admin_navigation_error(error_source, err))?;

    render_navigation_panel_html(&content, template_source)
}

async fn render_navigation_error_panel(
    state: &AdminState,
    status: NavigationListStatus,
    filter: &NavigationQueryFilter,
    cursor: Option<NavigationCursor>,
    cursor_state: &CursorState,
    message: impl Into<String>,
) -> Response {
    let mut content = match build_navigation_list_view(state, status, filter, cursor).await {
        Ok(content) => content,
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_error_panel", err)
                .into_response();
        }
    };

    apply_navigation_pagination_links(&mut content, cursor_state);

    let panel_html =
        match render_navigation_panel_html(&content, "infra::http::admin_navigation_error_panel") {
            Ok(html) => html,
            Err(err) => return err.into_response(),
        };

    let mut stream = datastar_replace(NAVIGATION_PANEL, panel_html);
    if let Err(err) = push_toasts(&mut stream, &[Toast::error(message.into())]) {
        return err.into_response();
    }

    stream.into_response()
}

fn render_navigation_editor_panel(
    content: &admin_views::AdminNavigationEditorView,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let template = admin_views::AdminNavigationEditPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}

async fn build_navigation_editor_view(
    state: &AdminState,
    item: Option<&NavigationItemRecord>,
    form: Option<&AdminNavigationForm>,
) -> Result<admin_views::AdminNavigationEditorView, HttpError> {
    let pages = state
        .navigation
        .published_pages()
        .await
        .map_err(|err| admin_navigation_error("infra::http::admin_navigation_editor", err))?;

    let destination_type = form
        .and_then(|f| parse_navigation_type(&f.destination_type).ok())
        .or_else(|| item.map(|i| i.destination_type))
        .unwrap_or(NavigationDestinationType::Internal);

    let destination_page_id = form
        .and_then(|f| parse_optional_uuid(f.destination_page_id.as_deref()))
        .or_else(|| item.and_then(|i| i.destination_page_id));

    let page_has_selection = destination_page_id.is_some();

    let destination_url = if destination_type == NavigationDestinationType::External {
        form.and_then(|f| blank_to_none_opt(f.destination_url.clone()))
            .or_else(|| item.and_then(|i| i.destination_url.clone()))
    } else {
        form.and_then(|f| blank_to_none_opt(f.destination_url.clone()))
    };

    let label = form
        .map(|f| f.label.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| item.map(|i| i.label.clone()))
        .unwrap_or_default();

    let sort_order = form
        .map(|f| f.sort_order)
        .or_else(|| item.map(|i| i.sort_order))
        .unwrap_or(0);

    let visible = form
        .map(|f| f.visible.is_some())
        .or_else(|| item.map(|i| i.visible))
        .unwrap_or(true);

    let open_in_new_tab = form
        .map(|f| f.open_in_new_tab.is_some())
        .or_else(|| item.map(|i| i.open_in_new_tab))
        .unwrap_or(false);

    let destination_type_options = navigation_destination_options(destination_type);

    let mut page_options: Vec<admin_views::AdminNavigationPageOption> = pages
        .into_iter()
        .map(|page| admin_views::AdminNavigationPageOption {
            id: page.id.to_string(),
            title: page.title,
            slug: page.slug,
            selected: Some(page.id) == destination_page_id,
        })
        .collect();
    page_options.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

    let id_value = item.map(|i| i.id.to_string());
    let toggle_suffix = id_value.as_deref().unwrap_or("new");
    let visible_input_id = format!("navigation-visible-{}", toggle_suffix);
    let open_in_new_tab_input_id = format!("navigation-open-in-new-tab-{}", toggle_suffix);

    Ok(admin_views::AdminNavigationEditorView {
        heading: match item {
            Some(item) => format!("Edit Navigation Item: {}", item.label),
            None => "Create Navigation Item".to_string(),
        },
        id: id_value,
        label,
        destination_type_options,
        page_options,
        destination_url,
        sort_order,
        visible,
        open_in_new_tab,
        page_has_selection,
        form_action: item
            .map(|i| format!("/navigation/{}/edit", i.id))
            .unwrap_or_else(|| "/navigation/create".to_string()),
        submit_label: if item.is_some() {
            "Save Changes".to_string()
        } else {
            "Create Item".to_string()
        },
        enable_live_submit: true,
        active_destination_type: navigation_type_key(destination_type).to_string(),
        preview_action: item
            .map(|i| format!("/navigation/{}/destination-preview", i.id))
            .unwrap_or_else(|| "/navigation/destination-preview".to_string()),
        visible_input_id,
        open_in_new_tab_input_id,
    })
}

fn map_navigation_row(
    item: &NavigationItemRecord,
    public_site_url: &str,
) -> admin_views::AdminNavigationRowView {
    let preview_href = match item.destination_type {
        NavigationDestinationType::Internal => item
            .destination_page_slug
            .as_deref()
            .map(|slug| format!("{}{}", public_site_url, slug))
            .unwrap_or_else(|| public_site_url.to_string()),
        NavigationDestinationType::External => item
            .destination_url
            .as_deref()
            .map(|url| url.to_string())
            .unwrap_or_else(|| "#".to_string()),
    };

    let destination_type_label = navigation_type_label(item.destination_type).to_string();
    let destination_display = match item.destination_type {
        NavigationDestinationType::Internal => item
            .destination_page_slug
            .as_deref()
            .map(|slug| format!("/{slug}"))
            .unwrap_or_else(|| "—".to_string()),
        NavigationDestinationType::External => item
            .destination_url
            .as_deref()
            .map(|url| url.to_string())
            .unwrap_or_else(|| "—".to_string()),
    };

    let destination_type_status = navigation_type_key(item.destination_type).to_string();
    let toggle_label = if item.visible { "Hide" } else { "Show" };

    admin_views::AdminNavigationRowView {
        id: item.id.to_string(),
        label: item.label.clone(),
        preview_href,
        destination_type_label,
        destination_type_status,
        destination_display,
        sort_order: item.sort_order,
        visible: item.visible,
        toggle_action: format!("/navigation/{}/visibility", item.id),
        toggle_label,
        edit_href: format!("/navigation/{}/edit", item.id),
        delete_action: format!("/navigation/{}/delete", item.id),
    }
}

fn build_navigation_filter(search: Option<&str>) -> NavigationQueryFilter {
    NavigationQueryFilter {
        search: search
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
    }
}

fn parse_navigation_status(value: Option<&str>) -> Result<NavigationListStatus, HttpError> {
    match value.unwrap_or("all").to_ascii_lowercase().as_str() {
        "all" => Ok(NavigationListStatus::All),
        "visible" => Ok(NavigationListStatus::Visible),
        "hidden" => Ok(NavigationListStatus::Hidden),
        other => Err(HttpError::new(
            "infra::http::admin_navigation_status",
            StatusCode::BAD_REQUEST,
            "Unknown navigation status",
            format!("Status `{other}` is not recognised"),
        )),
    }
}

fn status_filters(
    counts: &NavigationStatusCounts,
    active: NavigationListStatus,
) -> Vec<admin_views::AdminNavigationStatusFilterView> {
    [
        (NavigationListStatus::All, counts.total),
        (NavigationListStatus::Visible, counts.visible),
        (NavigationListStatus::Hidden, counts.hidden),
    ]
    .into_iter()
    .map(
        |(status, count)| admin_views::AdminNavigationStatusFilterView {
            status_key: status_key(status).map(|key| key.to_string()),
            label: status_label(status).to_string(),
            count,
            is_active: status == active,
        },
    )
    .collect()
}

fn status_key(status: NavigationListStatus) -> Option<&'static str> {
    match status {
        NavigationListStatus::All => None,
        NavigationListStatus::Visible => Some("visible"),
        NavigationListStatus::Hidden => Some("hidden"),
    }
}

fn status_label(status: NavigationListStatus) -> &'static str {
    match status {
        NavigationListStatus::All => "All",
        NavigationListStatus::Visible => "Visible",
        NavigationListStatus::Hidden => "Hidden",
    }
}

impl NavigationListStatus {
    fn visibility(self) -> Option<bool> {
        match self {
            NavigationListStatus::All => None,
            NavigationListStatus::Visible => Some(true),
            NavigationListStatus::Hidden => Some(false),
        }
    }
}

fn navigation_destination_options(
    selected: NavigationDestinationType,
) -> Vec<admin_views::AdminNavigationDestinationTypeOption> {
    [
        (NavigationDestinationType::Internal, "internal", "Internal"),
        (NavigationDestinationType::External, "external", "External"),
    ]
    .into_iter()
    .map(
        |(value, key, label)| admin_views::AdminNavigationDestinationTypeOption {
            value: key,
            label,
            selected: value == selected,
        },
    )
    .collect()
}

fn navigation_type_label(destination: NavigationDestinationType) -> &'static str {
    match destination {
        NavigationDestinationType::Internal => "Internal",
        NavigationDestinationType::External => "External",
    }
}

fn navigation_type_key(destination: NavigationDestinationType) -> &'static str {
    match destination {
        NavigationDestinationType::Internal => "internal",
        NavigationDestinationType::External => "external",
    }
}

fn normalize_public_site_url(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    }
}

fn parse_navigation_type(value: &str) -> Result<NavigationDestinationType, HttpError> {
    match value.to_ascii_lowercase().as_str() {
        "internal" => Ok(NavigationDestinationType::Internal),
        "external" => Ok(NavigationDestinationType::External),
        other => Err(HttpError::new(
            "infra::http::parse_navigation_type",
            StatusCode::BAD_REQUEST,
            "Unknown navigation destination type",
            format!("Destination type `{other}` is not recognised"),
        )),
    }
}

fn parse_optional_uuid(value: Option<&str>) -> Option<Uuid> {
    value.and_then(|raw| Uuid::parse_str(raw).ok())
}

fn admin_navigation_error(source: &'static str, err: AdminNavigationError) -> HttpError {
    match err {
        AdminNavigationError::ConstraintViolation(field) => HttpError::new(
            source,
            StatusCode::BAD_REQUEST,
            "Navigation request could not be processed",
            format!("Invalid field `{field}`"),
        ),
        AdminNavigationError::Repo(repo) => HttpError::from_error(
            source,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error",
            &repo,
        ),
    }
}
