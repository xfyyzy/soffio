use axum::http::StatusCode;

use crate::{
    application::{admin::posts::AdminPostStatusCounts, error::HttpError},
    domain::types::PostStatus,
    presentation::admin::views as admin_views,
};

pub(super) fn parse_post_status(value: Option<&str>) -> Result<Option<PostStatus>, HttpError> {
    let Some(raw) = value else {
        return Ok(None);
    };

    if raw.is_empty() {
        return Ok(None);
    }

    match raw.to_ascii_lowercase().as_str() {
        "draft" => Ok(Some(PostStatus::Draft)),
        "published" => Ok(Some(PostStatus::Published)),
        "archived" => Ok(Some(PostStatus::Archived)),
        "error" => Ok(Some(PostStatus::Error)),
        other => Err(HttpError::new(
            "infra::http::parse_post_status",
            StatusCode::BAD_REQUEST,
            "Unknown status filter",
            format!("Status `{other}` is not recognised"),
        )),
    }
}

pub(super) fn status_filters(
    counts: &AdminPostStatusCounts,
    active: Option<PostStatus>,
) -> Vec<admin_views::AdminPostStatusFilterView> {
    let mut filters = Vec::new();

    filters.push(admin_views::AdminPostStatusFilterView {
        status_key: None,
        label: "All".to_string(),
        count: counts.total,
        is_active: active.is_none(),
    });

    for status in [
        PostStatus::Draft,
        PostStatus::Published,
        PostStatus::Archived,
        PostStatus::Error,
    ] {
        let count = match status {
            PostStatus::Draft => counts.draft,
            PostStatus::Published => counts.published,
            PostStatus::Archived => counts.archived,
            PostStatus::Error => counts.error,
        };
        filters.push(admin_views::AdminPostStatusFilterView {
            status_key: Some(status_key(status).to_string()),
            label: status_label(status).to_string(),
            count,
            is_active: active == Some(status),
        });
    }

    filters
}

pub(super) fn post_status_options(selected: PostStatus) -> Vec<admin_views::AdminPostStatusOption> {
    [
        (PostStatus::Draft, "draft", "Draft"),
        (PostStatus::Published, "published", "Published"),
    ]
    .into_iter()
    .map(
        |(status, value, label)| admin_views::AdminPostStatusOption {
            value,
            label,
            selected: status == selected,
        },
    )
    .collect()
}

pub(super) fn status_key(status: PostStatus) -> &'static str {
    match status {
        PostStatus::Draft => "draft",
        PostStatus::Published => "published",
        PostStatus::Archived => "archived",
        PostStatus::Error => "error",
    }
}

pub(super) fn status_label(status: PostStatus) -> &'static str {
    match status {
        PostStatus::Draft => "Draft",
        PostStatus::Published => "Published",
        PostStatus::Archived => "Archived",
        PostStatus::Error => "Error",
    }
}
