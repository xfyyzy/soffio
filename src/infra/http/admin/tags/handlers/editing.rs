use super::*;

pub(crate) async fn admin_tag_new(State(state): State<AdminState>) -> Response {
    let chrome = match state.chrome.load("/tags").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let content = build_new_tag_view();
    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminTagEditTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_tag_edit(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
) -> Response {
    let chrome = match state.chrome.load("/tags").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let tag = match state.tags.find_by_id(id).await {
        Ok(Some(tag)) => tag,
        Ok(None) => return Redirect::to("/tags").into_response(),
        Err(err) => return admin_tag_error("infra::http::admin_tag_edit", err).into_response(),
    };

    let content = build_tag_edit_view(&tag);

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminTagEditTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_tag_update(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminTagForm>,
) -> Response {
    let actor = "admin";

    let command = UpdateTagCommand {
        id,
        name: form.name.trim().to_string(),
        description: blank_to_none_opt(form.description),
        pinned: shared::parse_checkbox_flag(&form.pinned),
    };

    let updated = match state.tags.update_tag(actor, command).await {
        Ok(tag) => tag,
        Err(err) => return admin_tag_error("infra::http::admin_tag_update", err).into_response(),
    };

    let content = build_tag_edit_view(&updated);

    let template = admin_views::AdminTagEditPanelTemplate {
        content: content.clone(),
    };

    let panel_html = match template.render() {
        Ok(html) => html,
        Err(err) => {
            return template_render_http_error(
                "infra::http::admin_tag_update",
                "Template rendering failed",
                err,
            )
            .into_response();
        }
    };

    let mut stream = datastar_replace(PANEL, panel_html);
    if let Err(err) = push_toasts(
        &mut stream,
        &[Toast::success(format!("Saved tag \"{}\"", updated.name))],
    ) {
        return err.into_response();
    }

    stream.into_response()
}

pub(crate) async fn admin_tag_create(
    State(state): State<AdminState>,
    Form(form): Form<AdminTagForm>,
) -> Response {
    let pinned_filter = match parse_tag_status(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter =
        shared::build_tag_filter(form.filter_search.as_deref(), form.filter_month.as_deref());
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let actor = "admin";
    let pinned = shared::parse_checkbox_flag(&form.pinned);

    let command = CreateTagCommand {
        name: form.name.trim().to_string(),
        description: blank_to_none_opt(form.description.clone()),
        pinned,
    };

    match state.tags.create_tag(actor, command).await {
        Ok(tag) => {
            let toasts = [Toast::success(format!("Created tag \"{}\"", tag.name))];
            respond_with_tag_editor_success(
                &state,
                TagEditorSuccess {
                    tag: &tag,
                    pinned_filter,
                    filter: &filter,
                    cursor_state: cursor_state.clone(),
                    toasts: &toasts,
                    template_source: "infra::http::admin_tag_create",
                },
            )
            .await
        }
        Err(err) => {
            let message = Toast::error(format!("Failed to create tag: {}", err));
            shared::respond_with_tags_panel_internal(shared::TagPanelResponseParams {
                state: &state,
                pinned_filter,
                filter: &filter,
                cursor_state,
                toasts: &[message],
                error_source: "infra::http::admin_tag_create",
                template_source: "infra::http::admin_tag_create",
            })
            .await
        }
    }
}

struct TagEditorSuccess<'a> {
    tag: &'a TagRecord,
    pinned_filter: Option<bool>,
    filter: &'a TagQueryFilter,
    cursor_state: CursorState,
    toasts: &'a [Toast],
    template_source: &'static str,
}

async fn respond_with_tag_editor_success(
    state: &AdminState,
    params: TagEditorSuccess<'_>,
) -> Response {
    let TagEditorSuccess {
        tag,
        pinned_filter,
        filter,
        cursor_state,
        toasts,
        template_source,
    } = params;

    let editor_view = build_tag_edit_view(tag);

    let editor_template = admin_views::AdminTagEditPanelTemplate {
        content: editor_view.clone(),
    };

    let editor_html = match editor_template.render() {
        Ok(html) => html,
        Err(err) => {
            return template_render_http_error(template_source, "Template rendering failed", err)
                .into_response();
        }
    };

    let cursor = match cursor_state.decode_with(TagCursor::decode, template_source) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut list_content = match build_tag_list_view(state, pinned_filter, filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_tag_error(template_source, err).into_response(),
    };

    apply_pagination_links(&mut list_content, &cursor_state);

    let panel_html = match render_tag_panel_html(&list_content, template_source) {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    stream_editor_success(EditorSuccessRender {
        editor_html,
        panel_html,
        panel_selector: TAGS_PANEL,
        toasts,
        history_path: Some(format!("/tags/{}/edit", tag.id)),
    })
}
