use axum::http::StatusCode;

use crate::{
    application::{admin::tags::AdminTagStatusCounts, error::HttpError},
    presentation::admin::views as admin_views,
};

pub(super) fn parse_tag_status(value: Option<&str>) -> Result<Option<bool>, HttpError> {
    let Some(raw) = value else {
        return Ok(None);
    };

    if raw.is_empty() {
        return Ok(None);
    }

    match raw.to_ascii_lowercase().as_str() {
        "pinned" => Ok(Some(true)),
        "unpinned" => Ok(Some(false)),
        other => Err(HttpError::new(
            "infra::http::admin::tags::parse_tag_status",
            StatusCode::BAD_REQUEST,
            "Unknown status filter",
            format!("Status `{other}` is not recognised"),
        )),
    }
}

pub(super) fn tag_status_filters(
    counts: &AdminTagStatusCounts,
    active: Option<bool>,
) -> Vec<admin_views::AdminPageStatusFilterView> {
    let mut filters = vec![admin_views::AdminPageStatusFilterView {
        status_key: None,
        label: "All".to_string(),
        count: counts.total,
        is_active: active.is_none(),
    }];

    filters.extend([
        admin_views::AdminPageStatusFilterView {
            status_key: Some(tag_status_key(true).to_string()),
            label: tag_status_label(true).to_string(),
            count: counts.pinned,
            is_active: active == Some(true),
        },
        admin_views::AdminPageStatusFilterView {
            status_key: Some(tag_status_key(false).to_string()),
            label: tag_status_label(false).to_string(),
            count: counts.unpinned,
            is_active: active == Some(false),
        },
    ]);

    filters
}

pub(super) fn tag_status_key(pinned: bool) -> &'static str {
    if pinned { "pinned" } else { "unpinned" }
}

pub(super) fn tag_status_label(pinned: bool) -> &'static str {
    if pinned { "Pinned" } else { "Unpinned" }
}
