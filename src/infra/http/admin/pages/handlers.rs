use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    application::{
        admin::pages::{CreatePageCommand, UpdatePageContentCommand, UpdatePageStatusCommand},
        error::HttpError,
        pagination::PageCursor,
        repos::{PageQueryFilter, SettingsRepo},
    },
    domain::{entities::PageRecord, types::PageStatus},
    infra::http::admin::{
        AdminState,
        pagination::{self, CursorState},
        selectors::{PAGES_PANEL, PANEL},
        shared::{
            AdminPostQuery, EditorSuccessRender, Toast, datastar_replace, push_toasts,
            stream_editor_success, template_render_http_error,
        },
    },
    presentation::{admin::views as admin_views, views::render_template_response},
};

use super::{
    editor::{build_new_page_editor_view, build_page_editor_view},
    errors::admin_page_error,
    forms::{AdminPageDeleteForm, AdminPageForm, AdminPagePanelForm, AdminPageStatusActionForm},
    panel::{build_page_list_view, build_page_panel_html, render_page_panel_html},
    status::{page_status_label, parse_page_status},
};

fn normalize_filter_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn build_page_filter(search: Option<&str>, month: Option<&str>) -> PageQueryFilter {
    PageQueryFilter {
        search: normalize_filter_value(search),
        month: normalize_filter_value(month),
    }
}

pub(crate) async fn admin_pages(
    State(state): State<AdminState>,
    Query(query): Query<AdminPostQuery>,
) -> Response {
    let chrome = match state.chrome.load("/pages").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(query.cursor.clone(), query.trail.clone());

    let status = match parse_page_status(query.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter = build_page_filter(query.search.as_deref(), query.month.as_deref());

    let cursor = match cursor_state.decode_with(PageCursor::decode, "infra::http::admin_pages") {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut content = match build_page_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_page_error("infra::http::admin_pages", err).into_response(),
    };

    apply_pagination_links(&mut content, &cursor_state);

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminPagesTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_page_panel(
    State(state): State<AdminState>,
    Form(form): Form<AdminPagePanelForm>,
) -> Response {
    let status = match parse_page_status(form.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let cursor = match cursor_state.decode_with(PageCursor::decode, "infra::http::admin_page_panel")
    {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = if form.clear.is_some() {
        build_page_filter(None, None)
    } else {
        build_page_filter(form.search.as_deref(), form.month.as_deref())
    };

    let mut content = match build_page_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_page_error("infra::http::admin_page_panel", err).into_response(),
    };

    apply_pagination_links(&mut content, &cursor_state);

    let panel_html = match render_page_panel_html(&content, "infra::http::admin_page_panel") {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    datastar_replace(PAGES_PANEL, panel_html).into_response()
}

pub(crate) async fn admin_page_new(State(state): State<AdminState>) -> Response {
    let chrome = match state.chrome.load("/pages").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let content = build_new_page_editor_view();
    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminPageEditTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_page_edit(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
) -> Response {
    let chrome = match state.chrome.load("/pages").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let page = match state.pages.find_by_id(id).await {
        Ok(Some(page)) => page,
        Ok(None) => {
            return HttpError::new(
                "infra::http::admin_page_edit",
                StatusCode::NOT_FOUND,
                "Page not found",
                format!("Page `{id}` could not be found"),
            )
            .into_response();
        }
        Err(err) => return admin_page_error("infra::http::admin_page_edit", err).into_response(),
    };

    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => {
            return admin_page_error("infra::http::admin_page_edit", err.into()).into_response();
        }
    };

    let content = build_page_editor_view(&page, timezone);

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminPageEditTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_page_update(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPageForm>,
) -> Response {
    let page = match state.pages.find_by_id(id).await {
        Ok(Some(page)) => page,
        Ok(None) => {
            return HttpError::new(
                "infra::http::admin_page_update",
                StatusCode::NOT_FOUND,
                "Page not found",
                format!("Page `{id}` could not be found"),
            )
            .into_response();
        }
        Err(err) => return admin_page_error("infra::http::admin_page_update", err).into_response(),
    };

    let status = match parse_page_status(Some(&form.status)) {
        Ok(option) => option.unwrap_or(page.status),
        Err(err) => return err.into_response(),
    };

    let command = UpdatePageContentCommand {
        id: page.id,
        slug: page.slug.clone(),
        title: form.title.trim().to_string(),
        body_markdown: form.body_markdown.trim().to_string(),
    };

    let actor = "admin";

    let updated = match state.pages.update_page(actor, command).await {
        Ok(page) => page,
        Err(err) => return admin_page_error("infra::http::admin_page_update", err).into_response(),
    };

    let final_record = if updated.status != status {
        let status_command = UpdatePageStatusCommand {
            id: updated.id,
            status,
            scheduled_at: page.scheduled_at,
            published_at: page.published_at,
            archived_at: page.archived_at,
        };
        match state.pages.update_status(actor, status_command).await {
            Ok(page) => page,
            Err(err) => {
                return admin_page_error("infra::http::admin_page_update", err).into_response();
            }
        }
    } else {
        updated
    };

    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => {
            return admin_page_error("infra::http::admin_page_update", err.into()).into_response();
        }
    };

    let content = build_page_editor_view(&final_record, timezone);

    let template = admin_views::AdminPageEditPanelTemplate {
        content: content.clone(),
    };

    let panel_html = match template.render() {
        Ok(html) => html,
        Err(err) => {
            return template_render_http_error(
                "infra::http::admin_page_update",
                "Template rendering failed",
                err,
            )
            .into_response();
        }
    };

    let mut stream = datastar_replace(PANEL, panel_html);

    if let Err(err) = push_toasts(
        &mut stream,
        &[Toast::success(format!(
            "Saved page \"{}\"",
            final_record.title
        ))],
    ) {
        return err.into_response();
    }

    stream.into_response()
}

pub(crate) async fn admin_page_create(
    State(state): State<AdminState>,
    Form(form): Form<AdminPageForm>,
) -> Response {
    let status_filter = match parse_page_status(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter = build_page_filter(form.filter_search.as_deref(), form.filter_month.as_deref());

    let status_value = match parse_page_status(Some(form.status.as_str())) {
        Ok(Some(status)) => status,
        Ok(None) => PageStatus::Draft,
        Err(err) => return err.into_response(),
    };

    let title = form.title.trim().to_string();
    let body_markdown = form.body_markdown.trim().to_string();

    let command = CreatePageCommand {
        title: title.clone(),
        body_markdown: body_markdown.clone(),
        status: status_value,
        scheduled_at: None,
        published_at: None,
        archived_at: None,
    };

    let actor = "admin";

    match state.pages.create_page(actor, command).await {
        Ok(page) => {
            let mut toasts = Vec::new();
            toasts.push(Toast::success(format!("Created page \"{}\"", page.title)));

            respond_with_page_editor_success(
                &state,
                PageEditorSuccess {
                    page: &page,
                    status_filter,
                    filter: &filter,
                    toasts: &toasts,
                    template_source: "infra::http::admin_page_create",
                },
            )
            .await
        }
        Err(err) => {
            let message = Toast::error(format!("Failed to create page: {}", err));
            let cursor_state = CursorState::default();
            respond_with_pages_panel_message(
                &state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                "infra::http::admin_page_create",
                "infra::http::admin_page_create",
            )
            .await
        }
    }
}

pub(crate) async fn admin_page_delete(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPageDeleteForm>,
) -> Response {
    let status_filter = match parse_page_status(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter = build_page_filter(form.filter_search.as_deref(), form.filter_month.as_deref());

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let actor = "admin";

    let page = match state.pages.find_by_id(id).await {
        Ok(Some(page)) => page,
        Ok(None) => {
            let message = Toast::error("Page not found");
            return respond_with_pages_panel_message(
                &state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                "infra::http::admin_page_delete",
                "infra::http::admin_page_delete",
            )
            .await;
        }
        Err(err) => {
            let message = Toast::error(format!("Failed to load page: {}", err));
            return respond_with_pages_panel_message(
                &state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                "infra::http::admin_page_delete",
                "infra::http::admin_page_delete",
            )
            .await;
        }
    };

    if let Err(err) = state.pages.delete_page(actor, page.id, &page.slug).await {
        let message = Toast::error(format!("Failed to delete page: {}", err));
        return respond_with_pages_panel_message(
            &state,
            status_filter,
            &filter,
            &cursor_state,
            message,
            "infra::http::admin_page_delete",
            "infra::http::admin_page_delete",
        )
        .await;
    }

    let message = Toast::success(format!("Deleted page \"{}\"", page.title));
    respond_with_pages_panel_message(
        &state,
        status_filter,
        &filter,
        &cursor_state,
        message,
        "infra::http::admin_page_delete",
        "infra::http::admin_page_delete",
    )
    .await
}

pub(crate) async fn admin_page_publish(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPageStatusActionForm>,
) -> Response {
    handle_page_status_action(
        &state,
        id,
        form,
        PageStatus::Published,
        "infra::http::admin_page_publish",
        "infra::http::admin_page_publish",
    )
    .await
}

pub(crate) async fn admin_page_move_to_draft(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPageStatusActionForm>,
) -> Response {
    handle_page_status_action(
        &state,
        id,
        form,
        PageStatus::Draft,
        "infra::http::admin_page_move_to_draft",
        "infra::http::admin_page_move_to_draft",
    )
    .await
}

pub(crate) async fn admin_page_archive(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPageStatusActionForm>,
) -> Response {
    handle_page_status_action(
        &state,
        id,
        form,
        PageStatus::Archived,
        "infra::http::admin_page_archive",
        "infra::http::admin_page_archive",
    )
    .await
}

async fn handle_page_status_action(
    state: &AdminState,
    id: Uuid,
    form: AdminPageStatusActionForm,
    target_status: PageStatus,
    error_source: &'static str,
    template_source: &'static str,
) -> Response {
    let status_filter = match parse_page_status(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter = build_page_filter(form.filter_search.as_deref(), form.filter_month.as_deref());
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let page = match state.pages.find_by_id(id).await {
        Ok(Some(page)) => page,
        Ok(None) => {
            let message = Toast::error("Page not found");
            return respond_with_pages_panel_message(
                state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                error_source,
                template_source,
            )
            .await;
        }
        Err(err) => {
            let message = Toast::error(format!("Failed to load page: {}", err));
            return respond_with_pages_panel_message(
                state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                error_source,
                template_source,
            )
            .await;
        }
    };

    let actor = "admin";
    let command = UpdatePageStatusCommand {
        id: page.id,
        status: target_status,
        scheduled_at: page.scheduled_at,
        published_at: page.published_at,
        archived_at: page.archived_at,
    };

    let message = match state.pages.update_status(actor, command).await {
        Ok(updated) => Toast::success(format!(
            "Updated page \"{}\" to {}",
            updated.title,
            page_status_label(target_status),
        )),
        Err(err) => Toast::error(format!("Failed to update page status: {}", err)),
    };

    respond_with_pages_panel_message(
        state,
        status_filter,
        &filter,
        &cursor_state,
        message,
        error_source,
        template_source,
    )
    .await
}

async fn respond_with_pages_panel_message(
    state: &AdminState,
    status_filter: Option<PageStatus>,
    filter: &PageQueryFilter,
    cursor_state: &CursorState,
    message: Toast,
    error_source: &'static str,
    template_source: &'static str,
) -> Response {
    let messages = [message];
    respond_with_pages_panel_with_state(
        state,
        status_filter,
        filter,
        cursor_state,
        &messages,
        error_source,
        template_source,
    )
    .await
}

async fn respond_with_pages_panel_with_state(
    state: &AdminState,
    status_filter: Option<PageStatus>,
    filter: &PageQueryFilter,
    cursor_state: &CursorState,
    toasts: &[Toast],
    error_source: &'static str,
    template_source: &'static str,
) -> Response {
    let cursor = match cursor_state.decode_with(PageCursor::decode, error_source) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut content = match build_page_list_view(state, status_filter, filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_page_error(error_source, err).into_response(),
    };

    apply_pagination_links(&mut content, cursor_state);

    match render_page_panel_html(&content, template_source) {
        Ok(html) => {
            let mut stream = datastar_replace(PAGES_PANEL, html);
            if !toasts.is_empty()
                && let Err(err) = push_toasts(&mut stream, toasts)
            {
                return err.into_response();
            }
            stream.into_response()
        }
        Err(err) => err.into_response(),
    }
}

struct PageEditorSuccess<'a> {
    page: &'a PageRecord,
    status_filter: Option<PageStatus>,
    filter: &'a PageQueryFilter,
    toasts: &'a [Toast],
    template_source: &'static str,
}

async fn respond_with_page_editor_success(
    state: &AdminState,
    params: PageEditorSuccess<'_>,
) -> Response {
    let PageEditorSuccess {
        page,
        status_filter,
        filter,
        toasts,
        template_source,
    } = params;

    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => return admin_page_error(template_source, err.into()).into_response(),
    };

    let content = build_page_editor_view(page, timezone);

    let editor_template = admin_views::AdminPageEditPanelTemplate {
        content: content.clone(),
    };

    let editor_html = match editor_template.render() {
        Ok(html) => html,
        Err(err) => {
            return template_render_http_error(template_source, "Template rendering failed", err)
                .into_response();
        }
    };

    let panel_html = match build_page_panel_html(
        state,
        status_filter,
        filter,
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
        panel_selector: PAGES_PANEL,
        toasts,
        history_path: Some(format!("/pages/{}/edit", page.id)),
    })
}

fn apply_pagination_links(
    content: &mut admin_views::AdminPageListView,
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
