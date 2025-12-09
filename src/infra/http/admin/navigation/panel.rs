//! Navigation panel building and rendering functions.

use askama::Template;
use axum::http::StatusCode;
use url::form_urlencoded::Serializer;

use crate::application::admin::navigation::AdminNavigationError;
use crate::application::error::HttpError;
use crate::application::pagination::{NavigationCursor, PageRequest};
use crate::application::repos::{NavigationQueryFilter, SettingsRepo};
use crate::domain::entities::NavigationItemRecord;
use crate::domain::types::NavigationDestinationType;
use crate::infra::http::admin::AdminState;
use crate::infra::http::admin::pagination::{self, CursorState};
use crate::infra::http::admin::shared::template_render_http_error;
use crate::infra::http::repo_error_to_http;
use crate::presentation::admin::views as admin_views;

use super::status::{
    NavigationListStatus, navigation_type_key, navigation_type_label, normalize_public_site_url,
    status_filters, status_key,
};

pub(super) fn apply_navigation_pagination_links(
    content: &mut admin_views::AdminNavigationListView,
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

pub(super) async fn build_navigation_list_view(
    state: &AdminState,
    status: NavigationListStatus,
    filter: &NavigationQueryFilter,
    cursor: Option<NavigationCursor>,
) -> Result<admin_views::AdminNavigationListView, AdminNavigationError> {
    let settings = state.db.load_site_settings().await?;
    let admin_page_size = settings.admin_page_size.clamp(1, 100).max(1) as u32;
    let public_site_url = normalize_public_site_url(&settings.public_site_url);

    let counts_future = state.navigation.status_counts(filter);
    let list_future = state.navigation.list(
        status.visibility(),
        filter,
        PageRequest::new(admin_page_size, cursor),
    );

    let (counts, page) = tokio::try_join!(counts_future, list_future)?;

    let filters = status_filters(&counts, status);

    let items = page
        .items
        .into_iter()
        .map(|item| map_navigation_row(&item, &public_site_url))
        .collect();

    let mut serializer = Serializer::new(String::new());
    if let Some(search) = filter.search.as_ref() {
        serializer.append_pair("search", search);
    }
    let filter_query = serializer.finish();

    Ok(admin_views::AdminNavigationListView {
        heading: "Navigation".to_string(),
        filters,
        items,
        filter_search: filter.search.clone(),
        filter_tag: None,
        filter_month: None,
        filter_query,
        next_cursor: page.next_cursor,
        cursor_param: None,
        trail: None,
        previous_page_state: None,
        next_page_state: None,
        tag_options: Vec::new(),
        month_options: Vec::new(),
        tag_filter_enabled: false,
        month_filter_enabled: false,
        panel_action: "/navigation/panel".to_string(),
        active_status_key: status_key(status).map(|s| s.to_string()),
        new_navigation_href: "/navigation/new".to_string(),
        tag_filter_label: "Tag".to_string(),
        tag_filter_all_label: "All tags".to_string(),
        tag_filter_field: "tag".to_string(),
        custom_hidden_fields: Vec::new(), // Navigation has no special filters
    })
}

pub(super) fn render_navigation_panel_html(
    content: &admin_views::AdminNavigationListView,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let template = admin_views::AdminNavigationPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}

pub(super) async fn build_navigation_panel_html(
    state: &AdminState,
    status: NavigationListStatus,
    filter: &NavigationQueryFilter,
    error_source: &'static str,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let content = build_navigation_list_view(state, status, filter, None)
        .await
        .map_err(|err| admin_navigation_error(error_source, err))?;

    render_navigation_panel_html(&content, template_source)
}

fn map_navigation_row(
    item: &NavigationItemRecord,
    public_site_url: &str,
) -> admin_views::AdminNavigationRowView {
    let preview_href = match item.destination_type {
        NavigationDestinationType::Internal => item
            .destination_page_slug
            .as_deref()
            .map(|slug| format!("{}{}", public_site_url, slug))
            .unwrap_or_else(|| public_site_url.to_string()),
        NavigationDestinationType::External => item
            .destination_url
            .as_deref()
            .map(|url| url.to_string())
            .unwrap_or_else(|| "#".to_string()),
    };

    let destination_type_label = navigation_type_label(item.destination_type).to_string();
    let destination_display = match item.destination_type {
        NavigationDestinationType::Internal => item
            .destination_page_slug
            .as_deref()
            .map(|slug| format!("/{slug}"))
            .unwrap_or_else(|| "—".to_string()),
        NavigationDestinationType::External => item
            .destination_url
            .as_deref()
            .map(|url| url.to_string())
            .unwrap_or_else(|| "—".to_string()),
    };

    let destination_type_status = navigation_type_key(item.destination_type).to_string();
    let toggle_label = if item.visible { "Hide" } else { "Show" };

    admin_views::AdminNavigationRowView {
        id: item.id.to_string(),
        label: item.label.clone(),
        preview_href,
        destination_type_label,
        destination_type_status,
        destination_display,
        sort_order: item.sort_order,
        visible: item.visible,
        toggle_action: format!("/navigation/{}/visibility", item.id),
        toggle_label,
        edit_href: format!("/navigation/{}/edit", item.id),
        delete_action: format!("/navigation/{}/delete", item.id),
    }
}

pub(super) fn admin_navigation_error(source: &'static str, err: AdminNavigationError) -> HttpError {
    match err {
        AdminNavigationError::ConstraintViolation(field) => HttpError::new(
            source,
            StatusCode::BAD_REQUEST,
            "Navigation request could not be processed",
            format!("Invalid field `{field}`"),
        ),
        AdminNavigationError::Repo(repo) => repo_error_to_http(source, repo),
    }
}
