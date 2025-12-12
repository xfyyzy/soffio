//! Snapshot list panel for posts/pages.
use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    application::{
        admin::snapshots::SnapshotServiceError,
        error::HttpError,
        pagination::{CursorPage, PageRequest, SnapshotCursor},
        repos::{SettingsRepo, SnapshotFilter, SnapshotMonthCount, SnapshotRecord},
    },
    domain::types::SnapshotEntityType,
    infra::http::admin::{
        AdminState,
        pagination::{self, CursorState},
        selectors::PANEL,
        shared::{AdminPostQuery, blank_to_none_opt, datastar_replace, template_render_http_error},
    },
    presentation::admin::views as admin_views,
    presentation::views::render_template_response,
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

#[derive(Clone, Copy)]
enum SnapshotEntity {
    Post,
    Page,
}

impl SnapshotEntity {
    fn kind(self) -> SnapshotEntityType {
        match self {
            SnapshotEntity::Post => SnapshotEntityType::Post,
            SnapshotEntity::Page => SnapshotEntityType::Page,
        }
    }
    fn label(self) -> &'static str {
        match self {
            SnapshotEntity::Post => "Post",
            SnapshotEntity::Page => "Page",
        }
    }
    fn slug(self) -> &'static str {
        match self {
            SnapshotEntity::Post => "posts",
            SnapshotEntity::Page => "pages",
        }
    }
}

pub async fn admin_post_snapshots(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Query(query): Query<AdminPostQuery>,
) -> Response {
    render_snapshots(&state, SnapshotEntity::Post, id, query).await
}

pub async fn admin_page_snapshots(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Query(query): Query<AdminPostQuery>,
) -> Response {
    render_snapshots(&state, SnapshotEntity::Page, id, query).await
}

pub async fn admin_post_snapshots_panel(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<SnapshotPanelForm>,
) -> Response {
    render_snapshots_panel(&state, SnapshotEntity::Post, id, form).await
}

pub async fn admin_page_snapshots_panel(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<SnapshotPanelForm>,
) -> Response {
    render_snapshots_panel(&state, SnapshotEntity::Page, id, form).await
}

async fn render_snapshots(
    state: &AdminState,
    entity: SnapshotEntity,
    id: Uuid,
    query: AdminPostQuery,
) -> Response {
    let chrome = match state.chrome.load(entity.slug()).await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(query.cursor.clone(), query.trail.clone());
    let cursor = match cursor_state.decode_with(SnapshotCursor::decode, SOURCE) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = SnapshotFilter {
        entity_type: Some(entity.kind()),
        entity_id: Some(id),
        search: blank_to_none_opt(query.search.clone()),
        month: blank_to_none_opt(query.month.clone()),
    };

    match build_snapshot_view(state, entity, id, filter, cursor_state, cursor).await {
        Ok(content) => {
            let view = admin_views::AdminLayout::new(chrome, content);
            render_template_response(
                admin_views::AdminSnapshotsTemplate { view },
                axum::http::StatusCode::OK,
            )
        }
        Err(resp) => resp,
    }
}

async fn render_snapshots_panel(
    state: &AdminState,
    entity: SnapshotEntity,
    id: Uuid,
    form: SnapshotPanelForm,
) -> Response {
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let cursor = match cursor_state.decode_with(SnapshotCursor::decode, SOURCE) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut filter = SnapshotFilter {
        entity_type: Some(entity.kind()),
        entity_id: Some(id),
        search: blank_to_none_opt(form.search.clone()),
        month: blank_to_none_opt(form.month.clone()),
    };

    if form.clear.is_some() {
        filter.search = None;
        filter.month = None;
    }

    match build_snapshot_view(state, entity, id, filter, cursor_state, cursor).await {
        Ok(content) => match (AdminSnapshotsPanelTemplate { content }).render() {
            Ok(html) => datastar_replace(PANEL, html).into_response(),
            Err(err) => {
                template_render_http_error(SOURCE, "Template rendering failed", err).into_response()
            }
        },
        Err(resp) => resp,
    }
}

async fn build_snapshot_view(
    state: &AdminState,
    entity: SnapshotEntity,
    id: Uuid,
    filter: SnapshotFilter,
    cursor_state: CursorState,
    cursor: Option<SnapshotCursor>,
) -> Result<admin_views::AdminSnapshotListView, Response> {
    let page_request = PageRequest::new(admin_page_size(state).await, cursor);

    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => {
            return Err(HttpError::new(
                SOURCE,
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load settings",
                err.to_string(),
            )
            .into_response());
        }
    };

    let (page, month_counts) = match load_snapshots(state, &filter, page_request).await {
        Ok(res) => res,
        Err(err) => return Err(err),
    };

    let mut content = build_content(
        &filter,
        page,
        month_counts,
        entity.label(),
        entity.slug(),
        id,
        timezone,
        &cursor_state,
    );

    apply_pagination_links(&mut content, &cursor_state);
    Ok(content)
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
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Snapshot repository error",
            repo.to_string(),
        )
        .into_response(),
        Snapshot(inner) => HttpError::new(
            SOURCE,
            axum::http::StatusCode::BAD_REQUEST,
            "Snapshot validation failed",
            inner.to_string(),
        )
        .into_response(),
        App(app) => HttpError::new(
            SOURCE,
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Snapshot error",
            app.to_string(),
        )
        .into_response(),
        NotFound => HttpError::new(
            SOURCE,
            axum::http::StatusCode::NOT_FOUND,
            "Snapshot not found",
            "Snapshot not found".to_string(),
        )
        .into_response(),
    }
}
