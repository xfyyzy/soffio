use askama::Template;
use std::str::FromStr;

use crate::{
    application::{
        api_keys::ApiKeyIssued,
        pagination::ApiKeyCursor,
        repos::{ApiKeyPageRequest, ApiKeyQueryFilter, SettingsRepo},
    },
    domain::api_keys::ApiScope,
    infra::http::admin::{
        AdminState,
        pagination::{self, CursorState},
    },
    presentation::admin::views as admin_views,
};

use super::{
    editor::scope_options,
    errors::ApiKeyHttpError,
    status::{ApiKeyStatusCounts, api_key_status_filters},
};

pub async fn build_panel_view(
    state: &AdminState,
    status_filter: Option<crate::application::repos::ApiKeyStatusFilter>,
    filter: &ApiKeyQueryFilter,
    cursor_state: &CursorState,
    issued: Option<&ApiKeyIssued>,
) -> Result<admin_views::AdminApiKeyListView, ApiKeyHttpError> {
    let settings = state
        .db
        .load_site_settings()
        .await
        .map_err(ApiKeyHttpError::from_repo)?;
    let timezone = settings.timezone;

    let cursor = cursor_state
        .decode_with(ApiKeyCursor::decode, "admin_api_keys")
        .map_err(ApiKeyHttpError::from_http)?;

    let page_req = ApiKeyPageRequest {
        limit: settings.admin_page_size as u32,
        cursor,
    };

    // Build query filter with the status from parsed result
    let query_filter = ApiKeyQueryFilter {
        status: status_filter,
        scope: filter.scope,
        search: filter.search.clone(),
    };

    let page = state
        .api_keys
        .list_page(&query_filter, page_req)
        .await
        .map_err(ApiKeyHttpError::from_api)?;

    let keys: Vec<_> = page
        .items
        .into_iter()
        .map(|key| admin_views::AdminApiKeyRowView {
            id: key.id.to_string(),
            name: key.name,
            prefix: key.prefix,
            scopes: key
                .scopes
                .iter()
                .map(|s| admin_views::AdminApiScopeDisplay {
                    slug: s.as_str().to_string(),
                    label: s.display_name().to_string(),
                })
                .collect(),
            created_at: admin_views::format_timestamp(key.created_at, timezone),
            last_used_at: key
                .last_used_at
                .map(|t| admin_views::format_timestamp(t, timezone)),
            expires_at: key
                .expires_at
                .map(|t| admin_views::format_timestamp(t, timezone)),
            status: key.status.as_str().to_string(),
            status_label: key.status.display_name().to_string(),
            description: key.description,
            revoke_action: format!("/api-keys/{}/revoke", key.id),
            rotate_action: format!("/api-keys/{}/rotate", key.id),
            delete_action: format!("/api-keys/{}/delete", key.id),
        })
        .collect();

    let new_token = issued.as_ref().map(|i| i.token.clone());

    // Build status counts struct
    let counts = ApiKeyStatusCounts {
        total: page.total,
        active: page.active,
        revoked: page.revoked,
        expired: page.expired,
    };

    // Build status filters using the parsed status (not raw string)
    let status_filters = api_key_status_filters(&counts, status_filter);
    let active_status_key = status_filters
        .iter()
        .find(|f| f.is_active)
        .and_then(|f| f.status_key.clone());

    // Build pagination states
    let mut previous_history = cursor_state.clone_history();
    let previous_token = previous_history.pop();
    let previous_page_state = previous_token.map(|token| {
        let prev_cursor_value = pagination::decode_cursor_token(&token);
        let prev_trail = pagination::join_cursor_history(&previous_history);
        admin_views::AdminApiKeyPaginationState {
            cursor: prev_cursor_value,
            trail: prev_trail,
        }
    });

    let next_page_state = page.next_cursor.map(|c| {
        let mut next_history = cursor_state.clone_history();
        next_history.push(pagination::encode_cursor_token(
            cursor_state.current_token_ref(),
        ));
        let next_trail = pagination::join_cursor_history(&next_history);
        admin_views::AdminApiKeyPaginationState {
            cursor: Some(c.encode()),
            trail: next_trail,
        }
    });

    // Scope filter options from aggregated counts
    let mut scope_filter_options: Vec<admin_views::AdminPostTagOption> = page
        .scope_counts
        .iter()
        .map(|(scope, count)| admin_views::AdminPostTagOption {
            slug: scope.as_str().to_string(),
            name: scope.as_str().to_string(),
            count: *count,
        })
        .collect();
    scope_filter_options.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(admin_views::AdminApiKeyListView {
        heading: "API keys".to_string(),
        keys,
        create_action: "/api-keys/create".to_string(),
        new_key_href: "/api-keys/new".to_string(),
        panel_action: "/api-keys/panel".to_string(),
        filters: status_filters,
        active_status_key,
        filter_search: filter.search.clone(),
        filter_scope: filter.scope.map(|s| s.as_str().to_string()),
        filter_tag: filter.scope.map(|s| s.as_str().to_string()),
        filter_month: None,
        tag_filter_enabled: true,
        month_filter_enabled: false,
        tag_filter_label: "Scope".to_string(),
        tag_filter_all_label: "All scopes".to_string(),
        tag_filter_field: "scope".to_string(),
        tag_options: scope_filter_options,
        month_options: Vec::new(),
        cursor_param: cursor_state.current_token(),
        trail: pagination::join_cursor_history(cursor_state.history_tokens()),
        previous_page_state,
        next_page_state,
        available_scopes: scope_options(),
        new_token,
    })
}

pub fn render_panel_html(
    content: &admin_views::AdminApiKeyListView,
) -> Result<String, ApiKeyHttpError> {
    let template = admin_views::AdminApiKeysPanelTemplate {
        content: content.clone(),
    };
    template
        .render()
        .map_err(|err| ApiKeyHttpError::from_template(err, "admin::api_keys::panel"))
}

pub fn render_created_panel_html(
    content: &admin_views::AdminApiKeyCreatedView,
) -> Result<String, ApiKeyHttpError> {
    let template = admin_views::AdminApiKeyCreatedPanelTemplate {
        content: content.clone(),
    };
    template
        .render()
        .map_err(|err| ApiKeyHttpError::from_template(err, "admin::api_keys::created_panel"))
}

/// Build API key query filter with proper normalization.
pub fn build_api_key_filter(scope: Option<&str>, search: Option<&str>) -> ApiKeyQueryFilter {
    ApiKeyQueryFilter {
        status: None, // Status is handled separately via parse_api_key_status
        scope: normalize_filter_value(scope).and_then(|s| ApiScope::from_str(&s).ok()),
        search: normalize_filter_value(search),
    }
}

/// Normalize filter value: trim and convert empty strings to None.
/// Consistent with posts/pages/tags pattern.
fn normalize_filter_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
}
