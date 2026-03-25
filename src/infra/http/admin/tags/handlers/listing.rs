use super::*;

pub(crate) async fn admin_tags(
    State(state): State<AdminState>,
    Query(query): Query<AdminPostQuery>,
) -> Response {
    let chrome = match state.chrome.load("/tags").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(query.cursor.clone(), query.trail.clone());

    let pinned_filter = match parse_tag_status(query.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter = shared::build_tag_filter(query.search.as_deref(), query.month.as_deref());

    let cursor = match cursor_state.decode_with(TagCursor::decode, "infra::http::admin_tags") {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut content = match build_tag_list_view(&state, pinned_filter, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_tag_error("infra::http::admin_tags", err).into_response(),
    };

    apply_pagination_links(&mut content, &cursor_state);

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminTagsTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_tags_panel(
    State(state): State<AdminState>,
    Form(form): Form<AdminTagPanelForm>,
) -> Response {
    let pinned_filter = match parse_tag_status(form.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let cursor = match cursor_state.decode_with(TagCursor::decode, "infra::http::admin_tags_panel")
    {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = if form.clear.is_some() {
        shared::build_tag_filter(None, None)
    } else {
        shared::build_tag_filter(form.search.as_deref(), form.month.as_deref())
    };

    let mut content = match build_tag_list_view(&state, pinned_filter, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_tag_error("infra::http::admin_tags_panel", err).into_response(),
    };

    apply_pagination_links(&mut content, &cursor_state);

    let panel_html = match render_tag_panel_html(&content, "infra::http::admin_tags_panel") {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    datastar_replace(TAGS_PANEL, panel_html).into_response()
}
