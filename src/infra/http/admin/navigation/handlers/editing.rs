use super::*;

pub(crate) async fn admin_navigation_new(State(state): State<AdminState>) -> Response {
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

pub(crate) async fn admin_navigation_edit(
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

pub(crate) async fn admin_navigation_create(
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

pub(crate) async fn admin_navigation_update(
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

pub(crate) async fn admin_navigation_destination_preview(
    State(state): State<AdminState>,
    Form(form): Form<AdminNavigationForm>,
) -> Response {
    handle_editor_preview(&state, None, form, "infra::http::admin_navigation_preview").await
}

pub(crate) async fn admin_navigation_destination_preview_for_item(
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
