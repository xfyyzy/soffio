use super::*;

pub(crate) async fn admin_navigation(
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

pub(crate) async fn admin_navigation_panel(
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
