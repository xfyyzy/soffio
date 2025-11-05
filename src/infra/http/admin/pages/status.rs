use axum::http::StatusCode;

use crate::{
    application::{admin::pages::AdminPageStatusCounts, error::HttpError},
    domain::types::PageStatus,
    presentation::admin::views as admin_views,
};

pub(crate) fn parse_page_status(value: Option<&str>) -> Result<Option<PageStatus>, HttpError> {
    let Some(raw) = value else {
        return Ok(None);
    };

    if raw.is_empty() {
        return Ok(None);
    }

    match raw.to_ascii_lowercase().as_str() {
        "draft" => Ok(Some(PageStatus::Draft)),
        "published" => Ok(Some(PageStatus::Published)),
        "archived" => Ok(Some(PageStatus::Archived)),
        "error" => Ok(Some(PageStatus::Error)),
        other => Err(HttpError::new(
            "infra::http::parse_page_status",
            StatusCode::BAD_REQUEST,
            "Unknown status filter",
            format!("Status `{other}` is not recognised"),
        )),
    }
}

pub(crate) fn page_status_filters(
    counts: &AdminPageStatusCounts,
    active: Option<PageStatus>,
) -> Vec<admin_views::AdminPageStatusFilterView> {
    let mut filters = Vec::new();
    filters.push(admin_views::AdminPageStatusFilterView {
        status_key: None,
        label: "All".to_string(),
        count: counts.total,
        is_active: active.is_none(),
    });

    for status in [
        PageStatus::Draft,
        PageStatus::Published,
        PageStatus::Archived,
        PageStatus::Error,
    ] {
        let count = match status {
            PageStatus::Draft => counts.draft,
            PageStatus::Published => counts.published,
            PageStatus::Archived => counts.archived,
            PageStatus::Error => counts.error,
        };
        filters.push(admin_views::AdminPageStatusFilterView {
            status_key: Some(page_status_key(status).to_string()),
            label: page_status_label(status).to_string(),
            count,
            is_active: active == Some(status),
        });
    }

    filters
}

pub(crate) fn page_status_key(status: PageStatus) -> &'static str {
    match status {
        PageStatus::Draft => "draft",
        PageStatus::Published => "published",
        PageStatus::Archived => "archived",
        PageStatus::Error => "error",
    }
}

pub(crate) fn page_status_label(status: PageStatus) -> &'static str {
    match status {
        PageStatus::Draft => "Draft",
        PageStatus::Published => "Published",
        PageStatus::Archived => "Archived",
        PageStatus::Error => "Error",
    }
}

pub(crate) fn page_status_options(selected: PageStatus) -> Vec<admin_views::AdminPageStatusOption> {
    [
        (PageStatus::Draft, "draft", "Draft"),
        (PageStatus::Published, "published", "Published"),
    ]
    .into_iter()
    .map(
        |(status, value, label)| admin_views::AdminPageStatusOption {
            value,
            label,
            selected: status == selected,
        },
    )
    .collect()
}
