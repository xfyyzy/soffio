//! Snapshot list panel for admin (per-entity).
use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    application::{
        admin::snapshots::SnapshotServiceError,
        error::HttpError,
        pagination::{CursorPage, PageRequest, SnapshotCursor},
        repos::SettingsRepo,
        repos::{SnapshotFilter, SnapshotMonthCount, SnapshotRecord},
    },
    domain::types::SnapshotEntityType,
    infra::http::admin::{
        AdminState,
        pagination::{self, CursorState},
        selectors::PANEL,
        shared::{AdminPostQuery, datastar_replace, template_render_http_error},
    },
    presentation::admin::views as admin_views,
};

const SOURCE: &str = "infra::http::admin::snapshots";

#[derive(Template)]
#[template(path = "admin/snapshots_panel.html")]
struct AdminSnapshotsPanelTemplate {
    pub content: admin_views::AdminSnapshotListView,
}

#[derive(Debug, serde::Deserialize)]
pub struct SnapshotPanelForm {
    pub search: Option<String>,
    pub month: Option<String>,
    pub cursor: Option<String>,
    pub trail: Option<String>,
    pub clear: Option<String>,
}

pub async fn admin_entity_snapshots(
    State(state): State<AdminState>,
    Path((entity, id)): Path<(String, Uuid)>,
    Query(query): Query<AdminPostQuery>,
) -> Response {
    let chrome_path = match entity.as_str() {
        "posts" => "/posts",
        "pages" => "/pages",
        _ => return not_found(entity),
    };

    let chrome = match state.chrome.load(chrome_path).await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(query.cursor.clone(), query.trail.clone());
    let cursor = match cursor_state.decode_with(SnapshotCursor::decode, SOURCE) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let (entity_type, entity_label) = match entity.as_str() {
        "posts" => (SnapshotEntityType::Post, "Post"),
        "pages" => (SnapshotEntityType::Page, "Page"),
        _ => return not_found(entity),
    };

    let filter = SnapshotFilter {
        entity_type: Some(entity_type),
        entity_id: Some(id),
        search: query.search.clone(),
        month: query.month.clone(),
    };

    let page_request = PageRequest::new(admin_page_size(&state).await, cursor);

    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => {
            return HttpError::new(
                SOURCE,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load settings",
                err.to_string(),
            )
            .into_response();
        }
    };

    let (page, month_counts) = match load_snapshots(&state, &filter, page_request).await {
        Ok(res) => res,
        Err(err) => return err,
    };

    let content = build_content(
        &filter,
        page,
        month_counts,
        entity_label,
        entity_slug(entity_label),
        id,
        timezone,
        &cursor_state,
    );

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminSnapshotsTemplate { view })
}

pub async fn admin_entity_snapshots_panel(
    State(state): State<AdminState>,
    Path((entity, id)): Path<(String, Uuid)>,
    Form(form): Form<SnapshotPanelForm>,
) -> Response {
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let cursor = match cursor_state.decode_with(SnapshotCursor::decode, SOURCE) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let entity_type = match entity.as_str() {
        "posts" => SnapshotEntityType::Post,
        "pages" => SnapshotEntityType::Page,
        _ => return not_found(entity),
    };

    let filter = if form.clear.is_some() {
        SnapshotFilter {
            entity_type: Some(entity_type),
            entity_id: Some(id),
            search: None,
            month: None,
        }
    } else {
        SnapshotFilter {
            entity_type: Some(entity_type),
            entity_id: Some(id),
            search: form.search.clone(),
            month: form.month.clone(),
        }
    };

    let page_request = PageRequest::new(admin_page_size(&state).await, cursor);
    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => {
            return HttpError::new(
                SOURCE,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load settings",
                err.to_string(),
            )
            .into_response();
        }
    };

    let (page, month_counts) = match load_snapshots(&state, &filter, page_request).await {
        Ok(res) => res,
        Err(err) => return err,
    };

    let (entity_label, slug) = match entity.as_str() {
        "posts" => ("Post", "posts"),
        "pages" => ("Page", "pages"),
        _ => return not_found(entity),
    };

    let content = build_content(
        &filter,
        page,
        month_counts,
        entity_label,
        slug,
        id,
        timezone,
        &cursor_state,
    );

    match (AdminSnapshotsPanelTemplate {
        content: content.clone(),
    })
    .render()
    {
        Ok(html) => datastar_replace(PANEL, html).into_response(),
        Err(err) => {
            template_render_http_error(SOURCE, "Template rendering failed", err).into_response()
        }
    }
}

async fn admin_page_size(state: &AdminState) -> u32 {
    match state.db.load_site_settings().await {
        Ok(settings) => settings.admin_page_size.clamp(1, 100).max(1) as u32,
        Err(_) => 20,
    }
}

async fn load_snapshots(
    state: &AdminState,
    filter: &SnapshotFilter,
    page: PageRequest<SnapshotCursor>,
) -> Result<(CursorPage<SnapshotRecord>, Vec<SnapshotMonthCount>), Response> {
    let page = state
        .snapshots
        .list(filter, page)
        .await
        .map_err(snapshot_error)?;
    let months = state
        .snapshots
        .month_counts(filter)
        .await
        .map_err(snapshot_error)?;
    Ok((page, months))
}

fn entity_slug(label: &str) -> &'static str {
    match label {
        "Post" => "posts",
        "Page" => "pages",
        _ => "entities",
    }
}

fn build_content(
    filter: &SnapshotFilter,
    page: CursorPage<SnapshotRecord>,
    month_counts: Vec<SnapshotMonthCount>,
    entity_label: &str,
    entity_slug: &str,
    id: Uuid,
    timezone: chrono_tz::Tz,
    cursor_state: &CursorState,
) -> admin_views::AdminSnapshotListView {
    let rows = page
        .items
        .into_iter()
        .map(|record| admin_views::AdminSnapshotRowView {
            id: record.id.to_string(),
            version: record.version,
            description: record.description.clone(),
            created_at: admin_views::format_timestamp(record.created_at, timezone),
            created_by: record.created_by,
            edit_href: format!("/snapshots/{}/edit", record.id),
            rollback_action: format!("/snapshots/{}/rollback", record.id),
            delete_action: format!("/snapshots/{}/delete", record.id),
        })
        .collect();

    let month_options: Vec<admin_views::AdminPostMonthOption> = month_counts
        .into_iter()
        .map(|month| admin_views::AdminPostMonthOption {
            key: month.key,
            label: month.label,
            count: month.count,
        })
        .collect();

    let mut content = admin_views::AdminSnapshotListView {
        heading: format!("{entity_label} Snapshots"),
        entity_label: entity_label.to_string(),
        filter_search: filter.search.clone(),
        filter_tag: None,
        filter_month: filter.month.clone(),
        month_options,
        tag_options: Vec::new(),
        tag_filter_label: "Tag".to_string(),
        tag_filter_all_label: "All tags".to_string(),
        tag_filter_field: "tag".to_string(),
        tag_filter_enabled: false,
        month_filter_enabled: true,
        snapshots: rows,
        new_snapshot_href: format!("/{entity_slug}/{id}/snapshots/new"),
        panel_action: format!("/{entity_slug}/{id}/snapshots"),
        next_cursor: page.next_cursor,
        cursor_param: None,
        trail: None,
        previous_page_state: None,
        next_page_state: None,
        custom_hidden_fields: Vec::new(),
        active_status_key: None,
    };

    apply_pagination_links(&mut content, cursor_state);
    content
}

fn apply_pagination_links(
    content: &mut admin_views::AdminSnapshotListView,
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

fn snapshot_error(err: SnapshotServiceError) -> Response {
    use SnapshotServiceError::*;
    match err {
        Repo(repo) => HttpError::new(
            SOURCE,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Snapshot repository error",
            repo.to_string(),
        )
        .into_response(),
        Snapshot(inner) => HttpError::new(
            SOURCE,
            StatusCode::BAD_REQUEST,
            "Snapshot validation failed",
            inner.to_string(),
        )
        .into_response(),
        App(app) => HttpError::new(
            SOURCE,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Snapshot error",
            app.to_string(),
        )
        .into_response(),
        NotFound => HttpError::new(
            SOURCE,
            StatusCode::NOT_FOUND,
            "Snapshot not found",
            "Snapshot not found".to_string(),
        )
        .into_response(),
    }
}

fn not_found(entity: String) -> Response {
    HttpError::new(
        SOURCE,
        StatusCode::NOT_FOUND,
        "Unsupported entity",
        format!("entity `{entity}` not supported"),
    )
    .into_response()
}

fn render_template_response<T: Template>(template: T) -> Response {
    match template.render() {
        Ok(html) => axum::response::Html(html).into_response(),
        Err(err) => {
            template_render_http_error(SOURCE, "Template rendering failed", err).into_response()
        }
    }
}
