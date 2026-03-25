use super::*;

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

    let filter =
        shared::build_page_filter(form.filter_search.as_deref(), form.filter_month.as_deref());

    let status_value = match parse_page_status(Some(form.status.as_str())) {
        Ok(Some(status)) => status,
        Ok(None) => PageStatus::Draft,
        Err(err) => return err.into_response(),
    };

    let title = form.title.trim().to_string();
    let body_markdown = form.body_markdown.trim().to_string();

    let command = CreatePageCommand {
        slug: None,
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
            shared::respond_with_pages_panel_message(
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
