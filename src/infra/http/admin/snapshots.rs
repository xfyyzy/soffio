use askama::Template;
use axum::http::StatusCode;
use axum::{Form, extract::Path, extract::Query, extract::State};
use serde::Deserialize;
use uuid::Uuid;

use crate::application::admin::snapshot_types::{
    PageSnapshotPayload, PageSnapshotSource, PostSnapshotPayload, PostSnapshotSource,
};
use crate::application::admin::snapshots::SnapshotServiceError;
use crate::application::error::HttpError;
use crate::application::pagination::{PageRequest, SnapshotCursor};
use crate::application::repos::{SettingsRepo, SnapshotFilter};
use crate::domain::types::SnapshotEntityType;
use crate::infra::http::admin::pagination::{self, CursorState};
use crate::infra::http::admin::{AdminState, shared::template_render_http_error};
use crate::presentation::admin::views as admin_views;

#[derive(Debug, Deserialize)]
pub struct SnapshotQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub trail: Option<String>,
}

pub(super) async fn admin_snapshots(
    State(state): State<AdminState>,
    Query(query): Query<SnapshotQuery>,
) -> Result<axum::response::Response, HttpError> {
    let view = build_snapshot_view(&state, &query).await?;
    let chrome = state.chrome.load("/snapshots").await?;
    let layout = admin_views::AdminLayout::new(chrome, view);
    let template = admin_views::AdminSnapshotsTemplate { view: layout };
    let body = template.render().map_err(|err| {
        template_render_http_error(
            "templates/admin/snapshots.html",
            "Template rendering failed",
            err,
        )
    })?;

    Ok(axum::response::Response::builder()
        .status(StatusCode::OK)
        .body(axum::body::Body::from(body))
        .unwrap())
}

pub(super) async fn admin_snapshots_panel(
    State(state): State<AdminState>,
    Form(query): Form<SnapshotQuery>,
) -> Result<axum::response::Response, HttpError> {
    let content = build_snapshot_view(&state, &query).await?;
    let body = (admin_views::AdminSnapshotsPanelTemplate {
        content,
        panel_action: "/snapshots/panel".to_string(),
    })
    .render()
    .map_err(|err| {
        template_render_http_error(
            "templates/admin/snapshots_panel.html",
            "Template rendering failed",
            err,
        )
    })?;

    Ok(axum::response::Response::builder()
        .status(StatusCode::OK)
        .body(axum::body::Body::from(body))
        .unwrap())
}

pub(super) async fn admin_snapshot_rollback(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, HttpError> {
    let actor = "admin"; // TODO: use authenticated admin identity when available

    let snapshot = state
        .snapshots
        .find(id)
        .await
        .map_err(snapshot_error_to_http)?
        .ok_or_else(|| {
            HttpError::new(
                "admin::snapshots",
                StatusCode::NOT_FOUND,
                "Snapshot not found",
                "snapshot not found",
            )
        })?;

    match snapshot.entity_type {
        SnapshotEntityType::Post => {
            state
                .snapshots
                .rollback::<PostSnapshotSource, _, _>(actor, id, |payload| async {
                    let post_payload: PostSnapshotPayload = payload;
                    state
                        .posts
                        .restore_from_snapshot(post_payload, snapshot.entity_id)
                        .await
                        .map(|_| ())
                        .map_err(post_snapshot_error)
                })
                .await
                .map_err(snapshot_error_to_http)?;
        }
        SnapshotEntityType::Page => {
            state
                .snapshots
                .rollback::<PageSnapshotSource, _, _>(actor, id, |payload| async {
                    let page_payload: PageSnapshotPayload = payload;
                    state
                        .pages
                        .restore_from_snapshot(page_payload, snapshot.entity_id)
                        .await
                        .map(|_| ())
                        .map_err(page_snapshot_error)
                })
                .await
                .map_err(snapshot_error_to_http)?;
        }
    }

    Ok(axum::response::Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(axum::http::header::LOCATION, "/snapshots")
        .body(axum::body::Body::empty())
        .unwrap())
}

async fn build_snapshot_view(
    state: &AdminState,
    query: &SnapshotQuery,
) -> Result<admin_views::AdminSnapshotListView, HttpError> {
    let entity_type = match query.entity_type.as_deref() {
        Some("post") => Some(SnapshotEntityType::Post),
        Some("page") => Some(SnapshotEntityType::Page),
        Some(other) => {
            return Err(HttpError::new(
                "admin::snapshots",
                StatusCode::BAD_REQUEST,
                "Invalid entity type",
                other.to_string(),
            ));
        }
        None => None,
    };

    let cursor = match query
        .cursor
        .as_deref()
        .map(SnapshotCursor::decode)
        .transpose()
    {
        Ok(c) => c,
        Err(err) => {
            return Err(HttpError::new(
                "admin::snapshots",
                StatusCode::BAD_REQUEST,
                "Invalid cursor",
                err.to_string(),
            ));
        }
    };

    let filter = SnapshotFilter {
        entity_type,
        entity_id: query.entity_id,
        search: query.search.clone(),
    };

    let cursor_state = CursorState::new(query.cursor.clone(), query.trail.clone());

    let page = state
        .snapshots
        .list(&filter, PageRequest::new(20, cursor))
        .await
        .map_err(snapshot_error_to_http)?;

    let tz = state
        .db
        .load_site_settings()
        .await
        .map_err(|err| {
            HttpError::new(
                "admin::snapshots",
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load settings",
                err.to_string(),
            )
        })?
        .timezone;

    let rows = page
        .items
        .into_iter()
        .map(|s| admin_views::AdminSnapshotRowView {
            id: s.id.to_string(),
            entity_type: format!("{:?}", s.entity_type).to_lowercase(),
            entity_id: s.entity_id.to_string(),
            version: s.version,
            description: s.description.clone(),
            schema_version: s.schema_version,
            created_at: admin_views::format_timestamp(s.created_at, tz),
            rollback_action: format!("/snapshots/{}/rollback", s.id),
        })
        .collect();

    let custom_hidden_fields = {
        let mut fields = Vec::new();
        if let Some(t) = &query.entity_type {
            fields.push(admin_views::AdminHiddenField::new("entity_type", t.clone()));
        }
        if let Some(id) = query.entity_id {
            fields.push(admin_views::AdminHiddenField::new(
                "entity_id",
                id.to_string(),
            ));
        }
        if let Some(search) = &query.search {
            fields.push(admin_views::AdminHiddenField::new("search", search.clone()));
        }
        fields
    };

    let cursor_param = cursor_state.current_token();
    let trail = pagination::join_cursor_history(cursor_state.history_tokens());

    let previous_page_state = {
        let mut history = cursor_state.clone_history();
        let previous_token = history.pop();
        previous_token.map(|token| admin_views::AdminPostPaginationState {
            cursor: pagination::decode_cursor_token(&token),
            trail: pagination::join_cursor_history(&history),
        })
    };

    let next_page_state = page.next_cursor.as_ref().map(|cursor| {
        let mut history = cursor_state.clone_history();
        history.push(pagination::encode_cursor_token(
            cursor_state.current_token_ref(),
        ));
        admin_views::AdminPostPaginationState {
            cursor: Some(cursor.clone()),
            trail: pagination::join_cursor_history(&history),
        }
    });

    Ok(admin_views::AdminSnapshotListView {
        heading: "Snapshots".to_string(),
        rows,
        filter_entity_type: query.entity_type.clone(),
        filter_search: query.search.clone(),
        filter_entity_id: query.entity_id.map(|v| v.to_string()),
        next_cursor: page.next_cursor,
        panel_action: "/snapshots/panel".to_string(),
        rollback_label: "Rollback",
        active_status_key: None,
        custom_hidden_fields,
        previous_page_state,
        next_page_state,
        cursor_param,
        trail,
    })
}

fn snapshot_error_to_http(err: SnapshotServiceError) -> HttpError {
    HttpError::new(
        "admin::snapshots",
        StatusCode::BAD_REQUEST,
        "Snapshot operation failed",
        err.to_string(),
    )
}

fn post_snapshot_error(
    err: crate::application::admin::posts::types::AdminPostError,
) -> SnapshotServiceError {
    match err {
        crate::application::admin::posts::types::AdminPostError::ConstraintViolation(field) => {
            SnapshotServiceError::Snapshot(crate::domain::snapshots::SnapshotError::Validation(
                field.to_string(),
            ))
        }
        crate::application::admin::posts::types::AdminPostError::Repo(repo) => {
            SnapshotServiceError::Repo(repo)
        }
    }
}

fn page_snapshot_error(
    err: crate::application::admin::pages::AdminPageError,
) -> SnapshotServiceError {
    match err {
        crate::application::admin::pages::AdminPageError::ConstraintViolation(field) => {
            SnapshotServiceError::Snapshot(crate::domain::snapshots::SnapshotError::Validation(
                field.to_string(),
            ))
        }
        crate::application::admin::pages::AdminPageError::Render(render_err) => {
            SnapshotServiceError::App(crate::application::error::AppError::unexpected(
                render_err.to_string(),
            ))
        }
        crate::application::admin::pages::AdminPageError::Repo(repo) => {
            SnapshotServiceError::Repo(repo)
        }
    }
}
