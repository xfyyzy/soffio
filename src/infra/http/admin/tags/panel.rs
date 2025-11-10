use askama::Template;
use url::form_urlencoded::Serializer;

use crate::{
    application::{
        admin::tags::AdminTagError,
        pagination::{PageRequest, TagCursor},
        repos::{SettingsRepo, TagQueryFilter},
    },
    infra::http::admin::{
        AdminState,
        pagination::{self, CursorState},
        shared::template_render_http_error,
    },
    presentation::admin::views as admin_views,
};

use super::status::{tag_status_filters, tag_status_key};

pub(super) async fn build_tag_list_view(
    state: &AdminState,
    pinned_filter: Option<bool>,
    filter: &TagQueryFilter,
    cursor: Option<TagCursor>,
) -> Result<admin_views::AdminTagListView, AdminTagError> {
    let settings = state.db.load_site_settings().await?;
    let admin_page_size = settings.admin_page_size.clamp(1, 100).max(1) as u32;
    let public_site_url = normalize_public_site_url(&settings.public_site_url);

    let counts_filter = filter.clone();
    let list_filter = filter.clone();
    let mut month_filter = filter.clone();
    month_filter.month = None;

    let page_request = PageRequest::new(admin_page_size, cursor);
    let (counts, page, month_counts) = tokio::try_join!(
        state.tags.status_counts(&counts_filter),
        state.tags.list(pinned_filter, &list_filter, page_request),
        state.tags.month_counts(pinned_filter, &month_filter)
    )?;

    let timezone = settings.timezone;

    let tags = page
        .items
        .into_iter()
        .map(|record| {
            let display_time_source = record.updated_at.unwrap_or(record.created_at);
            let display_time = admin_views::format_timestamp(display_time_source, timezone);
            let id_str = record.id.to_string();
            let slug = record.slug.clone();
            let public_href = format!("{}tags/{}", public_site_url, slug);
            admin_views::AdminTagRowView {
                id: id_str.clone(),
                name: record.name,
                slug,
                description: record.description,
                usage_count: record.usage_count,
                pinned: record.pinned,
                display_time: Some(display_time),
                display_time_kind: admin_views::AdminPostTimeKind::Updated,
                public_href,
                edit_href: format!("/tags/{id}/edit", id = id_str),
                pin_action: format!("/tags/{id}/pin", id = id_str),
                unpin_action: format!("/tags/{id}/unpin", id = id_str),
                delete_action: format!("/tags/{id}/delete", id = id_str),
            }
        })
        .collect();

    let filters = tag_status_filters(&counts, pinned_filter);

    let month_options = month_counts
        .into_iter()
        .map(|month| admin_views::AdminPostMonthOption {
            key: month.key,
            label: month.label,
            count: month.count,
        })
        .collect();

    let mut serializer = Serializer::new(String::new());
    if let Some(search) = filter.search.as_ref() {
        serializer.append_pair("search", search);
    }
    if let Some(month) = filter.month.as_ref() {
        serializer.append_pair("month", month);
    }
    let filter_query = serializer.finish();

    let active_status_key = pinned_filter.map(|value| tag_status_key(value).to_string());

    Ok(admin_views::AdminTagListView {
        heading: "Tags".to_string(),
        tags,
        filter_search: filter.search.clone(),
        filter_month: filter.month.clone(),
        filter_tag: None,
        filter_query,
        tag_options: Vec::new(),
        filters,
        month_options,
        next_cursor: page.next_cursor,
        cursor_param: None,
        trail: None,
        previous_page_state: None,
        next_page_state: None,
        active_status_key,
        panel_action: "/tags/panel".to_string(),
        new_tag_href: "/tags/new".to_string(),
        time_column_label: "Updated/Created".to_string(),
        month_filter_enabled: true,
        tag_filter_enabled: false,
        tag_filter_label: "Tag".to_string(),
        tag_filter_all_label: "All tags".to_string(),
        tag_filter_field: "tag".to_string(),
    })
}

pub(super) fn render_tag_panel_html(
    content: &admin_views::AdminTagListView,
    template_source: &'static str,
) -> Result<String, crate::application::error::HttpError> {
    let template = admin_views::AdminTagsPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}

pub(super) fn apply_pagination_links(
    content: &mut admin_views::AdminTagListView,
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

fn normalize_public_site_url(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{}/", url)
    }
}
