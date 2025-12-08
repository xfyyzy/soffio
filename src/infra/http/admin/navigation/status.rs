//! Navigation status helpers and filter types.

use axum::http::StatusCode;
use uuid::Uuid;

use crate::application::admin::navigation::NavigationStatusCounts;
use crate::application::error::HttpError;
use crate::application::repos::NavigationQueryFilter;
use crate::domain::types::NavigationDestinationType;
use crate::presentation::admin::views as admin_views;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NavigationListStatus {
    All,
    Visible,
    Hidden,
}

impl NavigationListStatus {
    pub(super) fn visibility(self) -> Option<bool> {
        match self {
            NavigationListStatus::All => None,
            NavigationListStatus::Visible => Some(true),
            NavigationListStatus::Hidden => Some(false),
        }
    }
}

pub(super) fn parse_navigation_status(
    value: Option<&str>,
) -> Result<NavigationListStatus, HttpError> {
    match value.unwrap_or("all").to_ascii_lowercase().as_str() {
        "all" => Ok(NavigationListStatus::All),
        "visible" => Ok(NavigationListStatus::Visible),
        "hidden" => Ok(NavigationListStatus::Hidden),
        other => Err(HttpError::new(
            "infra::http::admin_navigation_status",
            StatusCode::BAD_REQUEST,
            "Unknown navigation status",
            format!("Status `{other}` is not recognised"),
        )),
    }
}

pub(super) fn status_filters(
    counts: &NavigationStatusCounts,
    active: NavigationListStatus,
) -> Vec<admin_views::AdminNavigationStatusFilterView> {
    [
        (NavigationListStatus::All, counts.total),
        (NavigationListStatus::Visible, counts.visible),
        (NavigationListStatus::Hidden, counts.hidden),
    ]
    .into_iter()
    .map(
        |(status, count)| admin_views::AdminNavigationStatusFilterView {
            status_key: status_key(status).map(|key| key.to_string()),
            label: status_label(status).to_string(),
            count,
            is_active: status == active,
        },
    )
    .collect()
}

pub(super) fn status_key(status: NavigationListStatus) -> Option<&'static str> {
    match status {
        NavigationListStatus::All => None,
        NavigationListStatus::Visible => Some("visible"),
        NavigationListStatus::Hidden => Some("hidden"),
    }
}

fn status_label(status: NavigationListStatus) -> &'static str {
    match status {
        NavigationListStatus::All => "All",
        NavigationListStatus::Visible => "Visible",
        NavigationListStatus::Hidden => "Hidden",
    }
}

pub(super) fn build_navigation_filter(search: Option<&str>) -> NavigationQueryFilter {
    NavigationQueryFilter {
        search: search
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
    }
}

pub(super) fn parse_navigation_type(value: &str) -> Result<NavigationDestinationType, HttpError> {
    match value.to_ascii_lowercase().as_str() {
        "internal" => Ok(NavigationDestinationType::Internal),
        "external" => Ok(NavigationDestinationType::External),
        other => Err(HttpError::new(
            "infra::http::parse_navigation_type",
            StatusCode::BAD_REQUEST,
            "Unknown navigation destination type",
            format!("Destination type `{other}` is not recognised"),
        )),
    }
}

pub(super) fn parse_optional_uuid(value: Option<&str>) -> Option<Uuid> {
    value.and_then(|raw| Uuid::parse_str(raw).ok())
}

pub(super) fn navigation_destination_options(
    selected: NavigationDestinationType,
) -> Vec<admin_views::AdminNavigationDestinationTypeOption> {
    [
        (NavigationDestinationType::Internal, "internal", "Internal"),
        (NavigationDestinationType::External, "external", "External"),
    ]
    .into_iter()
    .map(
        |(value, key, label)| admin_views::AdminNavigationDestinationTypeOption {
            value: key,
            label,
            selected: value == selected,
        },
    )
    .collect()
}

pub(super) fn navigation_type_label(destination: NavigationDestinationType) -> &'static str {
    match destination {
        NavigationDestinationType::Internal => "Internal",
        NavigationDestinationType::External => "External",
    }
}

pub(super) fn navigation_type_key(destination: NavigationDestinationType) -> &'static str {
    match destination {
        NavigationDestinationType::Internal => "internal",
        NavigationDestinationType::External => "external",
    }
}

pub(super) fn normalize_public_site_url(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    }
}
