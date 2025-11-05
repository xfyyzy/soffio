use askama::Template;

use crate::{
    application::{
        admin::pages::AdminPageError,
        pagination::PageCursor,
        repos::{PageQueryFilter, SettingsRepo},
    },
    domain::types::PageStatus,
    infra::http::admin::{AdminState, shared::template_render_http_error},
    presentation::admin::views as admin_views,
};
use url::form_urlencoded::Serializer;

use super::{
    errors::admin_page_error,
    status::{page_status_filters, page_status_key, page_status_label},
};

pub(crate) async fn build_page_list_view(
    state: &AdminState,
    status: Option<PageStatus>,
    filter: &PageQueryFilter,
    cursor: Option<PageCursor>,
) -> Result<admin_views::AdminPageListView, AdminPageError> {
    let settings = state.db.load_site_settings().await?;
    let admin_page_size = settings.admin_page_size.clamp(1, 100).max(1) as u32;
    let public_site_url = normalize_public_site_url(&settings.public_site_url);

    let counts_filter = filter.clone();
    let mut month_filter = filter.clone();
    month_filter.month = None;
    let (counts, page, month_counts) = tokio::try_join!(
        state.pages.status_counts(&counts_filter),
        state.pages.list(status, admin_page_size, cursor, filter),
        state.pages.month_counts(status, &month_filter)
    )?;

    let pages = page
        .items
        .into_iter()
        .map(|record| {
            let (display_time, display_time_kind) = match record.status {
                PageStatus::Published => (
                    record
                        .published_at
                        .map(|time| admin_views::format_timestamp(time, settings.timezone)),
                    admin_views::AdminPostTimeKind::Published,
                ),
                _ => (
                    Some(admin_views::format_timestamp(
                        record.updated_at,
                        settings.timezone,
                    )),
                    admin_views::AdminPostTimeKind::Updated,
                ),
            };

            admin_views::AdminPageRowView {
                id: record.id.to_string(),
                title: record.title.clone(),
                slug: record.slug.clone(),
                status_key: page_status_key(record.status).to_string(),
                status_label: page_status_label(record.status).to_string(),
                display_time,
                display_time_kind,
                actions: page_actions_for_status(record.status),
                preview_href: format!("{}pages/_preview/{}", public_site_url, record.id),
                edit_href: format!("/pages/{}/edit", record.id),
            }
        })
        .collect();

    let filters = page_status_filters(&counts, status);

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

    Ok(admin_views::AdminPageListView {
        heading: "Pages".to_string(),
        filters,
        pages,
        filter_search: filter.search.clone(),
        filter_tag: None,
        filter_month: filter.month.clone(),
        filter_query,
        next_cursor: page.next_cursor,
        cursor_param: None,
        trail: None,
        previous_page_state: None,
        next_page_state: None,
        tag_options: Vec::new(),
        month_options,
        time_column_label: time_column_label(status),
        new_page_href: "/pages/new".to_string(),
        public_site_url,
        active_status_key: status.map(|s| page_status_key(s).to_string()),
        panel_action: "/pages/panel".to_string(),
        tag_filter_label: "Tag".to_string(),
        tag_filter_all_label: "All tags".to_string(),
        tag_filter_field: "tag".to_string(),
        tag_filter_enabled: false,
        month_filter_enabled: true,
        row_action_prefix: "/pages".to_string(),
    })
}

pub(super) fn render_page_panel_html(
    content: &admin_views::AdminPageListView,
    template_source: &'static str,
) -> Result<String, crate::application::error::HttpError> {
    let template = admin_views::AdminPagesPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}

pub(crate) async fn build_page_panel_html(
    state: &AdminState,
    status: Option<PageStatus>,
    filter: &PageQueryFilter,
    error_source: &'static str,
    template_source: &'static str,
) -> Result<String, crate::application::error::HttpError> {
    let content = build_page_list_view(state, status, filter, None)
        .await
        .map_err(|err| admin_page_error(error_source, err))?;

    render_page_panel_html(&content, template_source)
}

fn page_actions_for_status(status: PageStatus) -> Vec<admin_views::AdminPostRowActionView> {
    let mut actions = Vec::new();

    if status != PageStatus::Published {
        actions.push(admin_views::AdminPostRowActionView {
            value: "publish",
            label: "Publish",
            is_danger: false,
        });
    }
    if status != PageStatus::Draft {
        actions.push(admin_views::AdminPostRowActionView {
            value: "draft",
            label: "Move to Draft",
            is_danger: false,
        });
    }
    if status != PageStatus::Archived {
        actions.push(admin_views::AdminPostRowActionView {
            value: "archive",
            label: "Archive",
            is_danger: false,
        });
    }

    actions
}

fn normalize_public_site_url(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    }
}

fn time_column_label(status: Option<PageStatus>) -> String {
    match status {
        Some(PageStatus::Published) => "Published".to_string(),
        Some(_) => "Updated".to_string(),
        None => "Published/Updated".to_string(),
    }
}
