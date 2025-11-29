use axum::http::StatusCode;

use crate::{
    application::error::HttpError,
    application::repos::ApiKeyStatusFilter,
    domain::api_keys::ApiKeyStatus,
    presentation::admin::views as admin_views,
};

/// Parse API key status filter from query/form value.
/// Returns Ok(None) for empty strings (consistent with posts/pages/tags pattern).
pub fn parse_api_key_status(value: Option<&str>) -> Result<Option<ApiKeyStatusFilter>, HttpError> {
    let Some(raw) = value else {
        return Ok(None);
    };

    if raw.is_empty() {
        return Ok(None);
    }

    match raw.to_ascii_lowercase().as_str() {
        "active" => Ok(Some(ApiKeyStatusFilter::Active)),
        "revoked" => Ok(Some(ApiKeyStatusFilter::Revoked)),
        "expired" => Ok(Some(ApiKeyStatusFilter::Expired)),
        other => Err(HttpError::new(
            "infra::http::admin::api_keys::parse_api_key_status",
            StatusCode::BAD_REQUEST,
            "Unknown status filter",
            format!("Status `{other}` is not recognised"),
        )),
    }
}

/// Status counts for API keys panel tabs.
#[derive(Debug, Clone, Default)]
pub struct ApiKeyStatusCounts {
    pub total: u64,
    pub active: u64,
    pub revoked: u64,
    pub expired: u64,
}

/// Build status filter views for the panel tabs.
pub fn api_key_status_filters(
    counts: &ApiKeyStatusCounts,
    active: Option<ApiKeyStatusFilter>,
) -> Vec<admin_views::AdminApiKeyStatusFilterView> {
    vec![
        admin_views::AdminApiKeyStatusFilterView {
            status_key: None,
            label: "All".to_string(),
            count: counts.total,
            is_active: active.is_none(),
        },
        admin_views::AdminApiKeyStatusFilterView {
            status_key: Some(api_key_status_key(ApiKeyStatus::Active).to_string()),
            label: api_key_status_label(ApiKeyStatus::Active).to_string(),
            count: counts.active,
            is_active: active == Some(ApiKeyStatusFilter::Active),
        },
        admin_views::AdminApiKeyStatusFilterView {
            status_key: Some(api_key_status_key(ApiKeyStatus::Revoked).to_string()),
            label: api_key_status_label(ApiKeyStatus::Revoked).to_string(),
            count: counts.revoked,
            is_active: active == Some(ApiKeyStatusFilter::Revoked),
        },
        admin_views::AdminApiKeyStatusFilterView {
            status_key: Some(api_key_status_key(ApiKeyStatus::Expired).to_string()),
            label: api_key_status_label(ApiKeyStatus::Expired).to_string(),
            count: counts.expired,
            is_active: active == Some(ApiKeyStatusFilter::Expired),
        },
    ]
}

pub fn api_key_status_key(status: ApiKeyStatus) -> &'static str {
    match status {
        ApiKeyStatus::Active => "active",
        ApiKeyStatus::Revoked => "revoked",
        ApiKeyStatus::Expired => "expired",
    }
}

pub fn api_key_status_label(status: ApiKeyStatus) -> &'static str {
    match status {
        ApiKeyStatus::Active => "Active",
        ApiKeyStatus::Revoked => "Revoked",
        ApiKeyStatus::Expired => "Expired",
    }
}
