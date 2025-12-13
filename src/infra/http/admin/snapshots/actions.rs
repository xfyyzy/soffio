use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::application::admin::snapshots::SnapshotServiceError;
use crate::application::error::HttpError;
use crate::application::pagination::{PageRequest, SnapshotCursor};
use crate::application::repos::{SettingsRepo, SnapshotFilter};
use crate::domain::types::SnapshotEntityType;
use crate::infra::http::admin::{
    AdminState,
    pagination::CursorState,
    selectors::PANEL,
    shared::{Toast, blank_to_none_opt, datastar_replace, push_toasts, template_render_http_error},
};
use crate::presentation::admin::views as admin_views;
use askama::Template;

#[derive(Debug, serde::Deserialize)]
pub struct SnapshotActionForm {
    pub cursor: Option<String>,
    pub trail: Option<String>,
    pub search: Option<String>,
    pub month: Option<String>,
}

pub async fn admin_snapshot_rollback(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<SnapshotActionForm>,
) -> Response {
    handle_action(state, id, form, Action::Rollback).await
}

pub async fn admin_snapshot_delete(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<SnapshotActionForm>,
) -> Response {
    handle_action(state, id, form, Action::Delete).await
}

#[derive(Clone, Copy)]
enum Action {
    Rollback,
    Delete,
}

async fn handle_action(
    state: AdminState,
    id: Uuid,
    form: SnapshotActionForm,
    action: Action,
) -> Response {
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let cursor = match cursor_state.decode_with(SnapshotCursor::decode, SOURCE) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let actor = "admin:snapshots";
    let snapshot = match state.snapshots.find(id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return HttpError::new(
                SOURCE,
                StatusCode::NOT_FOUND,
                "Snapshot not found",
                "Snapshot not found".to_string(),
            )
            .into_response();
        }
        Err(err) => {
            return HttpError::new(
                SOURCE,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load snapshot",
                err.to_string(),
            )
            .into_response();
        }
    };

    let filter = SnapshotFilter {
        entity_type: Some(snapshot.entity_type),
        entity_id: Some(snapshot.entity_id),
        search: blank_to_none_opt(form.search.clone()),
        month: blank_to_none_opt(form.month.clone()),
    };

    let success_message = match action {
        Action::Rollback => format!("Rolled back snapshot v{}", snapshot.version),
        Action::Delete => format!("Deleted snapshot v{}", snapshot.version),
    };

    let page_request = PageRequest::new(admin_page_size(&state).await, cursor);
    let panel_result = match action {
        Action::Rollback => {
            do_rollback(&state, actor, id, snapshot.entity_type, snapshot.entity_id).await
        }
        Action::Delete => state.snapshots.delete(id).await.map(|_| ()),
    };

    match panel_result {
        Ok(()) => {
            load_panel(
                &state,
                &filter,
                cursor_state,
                page_request,
                &success_message,
                None,
            )
            .await
        }
        Err(err) => {
            load_panel(
                &state,
                &filter,
                cursor_state,
                page_request,
                &success_message,
                Some(err),
            )
            .await
        }
    }
    .unwrap_or_else(|resp| resp)
}

async fn do_rollback(
    state: &AdminState,
    actor: &str,
    snapshot_id: Uuid,
    entity_type: SnapshotEntityType,
    entity_id: Uuid,
) -> Result<(), SnapshotServiceError> {
    match entity_type {
        SnapshotEntityType::Post => {
            state
                .snapshots
                .rollback::<crate::application::admin::snapshot_types::PostSnapshotSource, _, _>(
                    actor,
                    snapshot_id,
                    |payload| async move {
                        state
                            .posts
                            .restore_from_snapshot(payload, entity_id)
                            .await
                            .map(|_| ())
                            .map_err(map_post_error)
                    },
                )
                .await?;
        }
        SnapshotEntityType::Page => {
            state
                .snapshots
                .rollback::<crate::application::admin::snapshot_types::PageSnapshotSource, _, _>(
                    actor,
                    snapshot_id,
                    |payload| async move {
                        state
                            .pages
                            .restore_from_snapshot(payload, entity_id)
                            .await
                            .map(|_| ())
                            .map_err(map_page_error)
                    },
                )
                .await?;
        }
    }
    Ok(())
}

fn map_post_error(
    err: crate::application::admin::posts::types::AdminPostError,
) -> SnapshotServiceError {
    use crate::application::admin::posts::types::AdminPostError;
    match err {
        AdminPostError::ConstraintViolation(field) => SnapshotServiceError::Snapshot(
            crate::domain::snapshots::SnapshotError::Validation(field.to_string()),
        ),
        AdminPostError::Repo(repo) => SnapshotServiceError::Repo(repo),
    }
}

fn map_page_error(err: crate::application::admin::pages::AdminPageError) -> SnapshotServiceError {
    use crate::application::admin::pages::AdminPageError;
    match err {
        AdminPageError::ConstraintViolation(field) => SnapshotServiceError::Snapshot(
            crate::domain::snapshots::SnapshotError::Validation(field.to_string()),
        ),
        AdminPageError::Render(render_err) => SnapshotServiceError::App(
            crate::application::error::AppError::unexpected(render_err.to_string()),
        ),
        AdminPageError::Repo(repo) => SnapshotServiceError::Repo(repo),
    }
}

async fn load_panel(
    state: &AdminState,
    filter: &SnapshotFilter,
    cursor_state: CursorState,
    page: PageRequest<SnapshotCursor>,
    success_message: &str,
    error: Option<SnapshotServiceError>,
) -> Result<Response, Response> {
    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => {
            return Err(HttpError::new(
                SOURCE,
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load settings",
                err.to_string(),
            )
            .into_response());
        }
    };

    let (page, month_counts) = match super::panel::load_snapshots(state, filter, page).await {
        Ok(res) => res,
        Err(err) => return Err(err),
    };

    let mut content = super::panel::build_content(
        filter,
        page,
        month_counts,
        entity_label(filter.entity_type.unwrap()),
        entity_slug(filter.entity_type.unwrap()),
        filter.entity_id.unwrap(),
        timezone,
        &cursor_state,
    );

    super::panel::apply_pagination_links(&mut content, &cursor_state);

    match (admin_views::AdminSnapshotsPanelTemplate { content }).render() {
        Ok(html) => {
            let mut stream = datastar_replace(PANEL, html);
            let toasts = match error {
                None => vec![Toast::success(success_message.to_string())],
                Some(err) => vec![Toast::error(error_to_message(&err))],
            };
            if let Err(err) = push_toasts(&mut stream, &toasts) {
                return Err(err.into_response());
            }
            Ok(stream.into_response())
        }
        Err(err) => Err(
            template_render_http_error(SOURCE, "Template rendering failed", err).into_response(),
        ),
    }
}

fn error_to_message(err: &SnapshotServiceError) -> String {
    match err {
        SnapshotServiceError::NotFound => "Snapshot not found".to_string(),
        SnapshotServiceError::Snapshot(inner) => format!("Snapshot validation failed: {}", inner),
        SnapshotServiceError::Repo(repo) => format!("Snapshot repository error: {}", repo),
        SnapshotServiceError::App(app) => format!("Snapshot error: {}", app),
    }
}

async fn admin_page_size(state: &AdminState) -> u32 {
    match state.db.load_site_settings().await {
        Ok(settings) => settings.admin_page_size.clamp(1, 100).max(1) as u32,
        Err(_) => 20,
    }
}

fn entity_slug(entity_type: SnapshotEntityType) -> &'static str {
    match entity_type {
        SnapshotEntityType::Post => "posts",
        SnapshotEntityType::Page => "pages",
    }
}

fn entity_label(entity_type: SnapshotEntityType) -> &'static str {
    match entity_type {
        SnapshotEntityType::Post => "Post",
        SnapshotEntityType::Page => "Page",
    }
}

const SOURCE: &str = "infra::http::admin::snapshots::actions";
