//! Shared utility functions for post handlers.

use crate::application::repos::PostQueryFilter;

pub(super) fn normalize_filter_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
}

pub(super) fn build_post_filter(
    search: Option<&str>,
    tag: Option<&str>,
    month: Option<&str>,
) -> PostQueryFilter {
    PostQueryFilter {
        search: normalize_filter_value(search),
        tag: normalize_filter_value(tag),
        month: normalize_filter_value(month),
    }
}

pub(super) fn parse_checkbox_flag(value: &Option<String>) -> bool {
    matches!(value.as_deref(), Some("true") | Some("on") | Some("1"))
}
