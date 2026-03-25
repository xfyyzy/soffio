use super::*;

fn normalize_filter_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

pub(super) fn build_page_filter(search: Option<&str>, month: Option<&str>) -> PageQueryFilter {
    PageQueryFilter {
        search: normalize_filter_value(search),
        month: normalize_filter_value(month),
    }
}

pub(super) async fn respond_with_pages_panel_message(
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

pub(super) fn apply_pagination_links(
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
