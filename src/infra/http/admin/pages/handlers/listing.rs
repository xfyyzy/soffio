use super::*;

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

    let filter = shared::build_page_filter(query.search.as_deref(), query.month.as_deref());

    let cursor = match cursor_state.decode_with(PageCursor::decode, "infra::http::admin_pages") {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut content = match build_page_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_page_error("infra::http::admin_pages", err).into_response(),
    };

    shared::apply_pagination_links(&mut content, &cursor_state);

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
        shared::build_page_filter(None, None)
    } else {
        shared::build_page_filter(form.search.as_deref(), form.month.as_deref())
    };

    let mut content = match build_page_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_page_error("infra::http::admin_page_panel", err).into_response(),
    };

    shared::apply_pagination_links(&mut content, &cursor_state);

    let panel_html = match render_page_panel_html(&content, "infra::http::admin_page_panel") {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    datastar_replace(PAGES_PANEL, panel_html).into_response()
}
