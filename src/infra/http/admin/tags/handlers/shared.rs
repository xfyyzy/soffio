use super::*;

fn normalize_filter_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
}

pub(super) fn build_tag_filter(search: Option<&str>, month: Option<&str>) -> TagQueryFilter {
    TagQueryFilter {
        search: normalize_filter_value(search),
        month: normalize_filter_value(month),
    }
}

pub(super) fn parse_checkbox_flag(input: &Option<String>) -> bool {
    matches!(input.as_deref(), Some("on") | Some("true"))
}

pub(super) struct TagPanelResponseParams<'a, 'b> {
    pub(super) state: &'a AdminState,
    pub(super) pinned_filter: Option<bool>,
    pub(super) filter: &'a TagQueryFilter,
    pub(super) cursor_state: CursorState,
    pub(super) toasts: &'b [Toast],
    pub(super) error_source: &'static str,
    pub(super) template_source: &'static str,
}

pub(super) async fn respond_with_tags_panel_internal(
    params: TagPanelResponseParams<'_, '_>,
) -> Response {
    let TagPanelResponseParams {
        state,
        pinned_filter,
        filter,
        cursor_state,
        toasts,
        error_source,
        template_source,
    } = params;

    let cursor = match cursor_state.decode_with(TagCursor::decode, error_source) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut content = match build_tag_list_view(state, pinned_filter, filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_tag_error(error_source, err).into_response(),
    };

    apply_pagination_links(&mut content, &cursor_state);

    match render_tag_panel_html(&content, template_source) {
        Ok(html) => {
            let mut stream = datastar_replace(TAGS_PANEL, html);
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

pub(super) trait IfEmptyExt {
    fn if_empty_then<F: FnOnce() -> String>(self, default: F) -> String;
}

impl IfEmptyExt for &str {
    fn if_empty_then<F: FnOnce() -> String>(self, default: F) -> String {
        if self.trim().is_empty() {
            default()
        } else {
            self.to_string()
        }
    }
}
