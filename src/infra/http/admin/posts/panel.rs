use askama::Template;

use crate::{
    application::{
        admin::posts::AdminPostError,
        error::HttpError,
        pagination::{PageRequest, PostCursor},
        repos::{PostQueryFilter, SettingsRepo},
    },
    domain::types::PostStatus,
    infra::http::admin::{AdminState, pagination::CursorState, shared::template_render_http_error},
    presentation::admin::views as admin_views,
};
use url::form_urlencoded::Serializer;

use super::{
    errors::admin_post_error,
    status::{status_filters, status_key, status_label},
};

pub(super) async fn build_post_list_view(
    state: &AdminState,
    status: Option<PostStatus>,
    filter: &PostQueryFilter,
    cursor: Option<PostCursor>,
) -> Result<admin_views::AdminPostListView, AdminPostError> {
    let settings = state.db.load_site_settings().await?;
    let admin_page_size = settings.admin_page_size.clamp(1, 100).max(1) as u32;
    let public_site_url = normalize_public_site_url(&settings.public_site_url);

    let counts_filter = filter.clone();
    let list_filter = filter.clone();
    let month_filter = PostQueryFilter {
        tag: filter.tag.clone(),
        month: None,
        search: filter.search.clone(),
    };
    let mut tag_count_filter = filter.clone();
    tag_count_filter.tag = None;

    let page_request = PageRequest::new(admin_page_size, cursor);
    let (counts, page, month_counts, tag_counts) = tokio::try_join!(
        state.posts.status_counts(&counts_filter),
        state.posts.list(status, &list_filter, page_request),
        state.posts.month_counts(status, &month_filter),
        state.posts.tag_counts(status, &tag_count_filter)
    )?;

    let posts = page
        .items
        .into_iter()
        .map(|post| {
            let preview_href = format!("{}posts/_preview/{}", public_site_url, post.id);
            let edit_href = format!("/posts/{}/edit", post.id);

            let (display_time, display_time_kind) = match post.status {
                PostStatus::Published => (
                    post.published_at
                        .map(|time| admin_views::format_timestamp(time, settings.timezone)),
                    admin_views::AdminPostTimeKind::Published,
                ),
                _ => (
                    Some(admin_views::format_timestamp(
                        post.updated_at,
                        settings.timezone,
                    )),
                    admin_views::AdminPostTimeKind::Updated,
                ),
            };

            admin_views::AdminPostRowView {
                id: post.id.to_string(),
                title: post.title,
                status_key: status_key(post.status).to_string(),
                status_label: status_label(post.status).to_string(),
                display_time,
                display_time_kind,
                actions: post_actions_for_status(post.status, post.pinned),
                preview_href,
                edit_href,
                is_pinned: post.pinned,
                snapshots_href: Some(format!("/posts/{}/snapshots", post.id)),
            }
        })
        .collect();

    let filters = status_filters(&counts, status);

    let tag_options = tag_counts
        .into_iter()
        .map(|tag| admin_views::AdminPostTagOption {
            slug: tag.slug,
            name: tag.name,
            count: tag.count,
        })
        .collect();

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
    if let Some(tag) = filter.tag.as_ref() {
        serializer.append_pair("tag", tag);
    }
    if let Some(month) = filter.month.as_ref() {
        serializer.append_pair("month", month);
    }
    let filter_query = serializer.finish();

    Ok(admin_views::AdminPostListView {
        heading: "Posts".to_string(),
        filters,
        posts,
        tag_options,
        month_options,
        filter_search: filter.search.clone(),
        filter_tag: filter.tag.clone(),
        filter_month: filter.month.clone(),
        next_cursor: page.next_cursor,
        filter_query,
        cursor_param: None,
        trail: None,
        previous_page_state: None,
        next_page_state: None,
        time_column_label: time_column_label(status),
        new_post_href: "/posts/new".to_string(),
        public_site_url,
        active_status_key: status.map(|s| status_key(s).to_string()),
        panel_action: "/posts/panel".to_string(),
        tag_filter_label: "Tag".to_string(),
        tag_filter_all_label: "All tags".to_string(),
        tag_filter_field: "tag".to_string(),
        tag_filter_enabled: true,
        month_filter_enabled: true,
        row_action_prefix: "/posts".to_string(),
        custom_hidden_fields: build_hidden_fields(filter),
    })
}

fn build_hidden_fields(filter: &PostQueryFilter) -> Vec<admin_views::AdminHiddenField> {
    let mut fields = Vec::new();
    if let Some(ref tag) = filter.tag {
        fields.push(admin_views::AdminHiddenField::new("tag", tag.clone()));
    }
    if let Some(ref month) = filter.month {
        fields.push(admin_views::AdminHiddenField::new("month", month.clone()));
    }
    fields
}

fn time_column_label(status: Option<PostStatus>) -> String {
    match status {
        Some(PostStatus::Published) => "Published".to_string(),
        Some(_) => "Updated".to_string(),
        None => "Published/Updated".to_string(),
    }
}

pub(super) fn render_post_panel_html(
    content: &admin_views::AdminPostListView,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let template = admin_views::AdminPostsPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}

pub(super) async fn build_post_panel_html(
    state: &AdminState,
    status: Option<PostStatus>,
    filter: &PostQueryFilter,
    error_source: &'static str,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let mut content = build_post_list_view(state, status, filter, None)
        .await
        .map_err(|err| admin_post_error(error_source, err))?;

    let cursor_state = CursorState::default();
    super::pagination::apply_pagination_links(&mut content, &cursor_state);

    render_post_panel_html(&content, template_source)
}

fn post_actions_for_status(
    status: PostStatus,
    pinned: bool,
) -> Vec<admin_views::AdminPostRowActionView> {
    let mut actions = Vec::new();

    if pinned {
        actions.push(admin_views::AdminPostRowActionView {
            value: "unpin",
            label: "Unpin",
            is_danger: false,
        });
    } else {
        actions.push(admin_views::AdminPostRowActionView {
            value: "pin",
            label: "Pin",
            is_danger: false,
        });
    }

    if status != PostStatus::Published {
        actions.push(admin_views::AdminPostRowActionView {
            value: "publish",
            label: "Publish",
            is_danger: false,
        });
    }
    if status != PostStatus::Draft {
        actions.push(admin_views::AdminPostRowActionView {
            value: "draft",
            label: "Move to Draft",
            is_danger: false,
        });
    }
    if status != PostStatus::Archived {
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
        format!("{}/", url)
    }
}
