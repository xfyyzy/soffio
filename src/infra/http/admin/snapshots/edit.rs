use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use uuid::Uuid;

use crate::application::{admin::snapshots::SnapshotServiceError, error::HttpError};
use crate::domain::types::SnapshotEntityType;
use crate::infra::http::admin::{AdminState, shared::blank_to_none_opt};
use crate::presentation::{admin::views as admin_views, views::render_template_response};

#[derive(Debug, serde::Deserialize)]
pub struct SnapshotEditForm {
    pub description: Option<String>,
}

pub async fn admin_snapshot_edit(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
) -> Response {
    let record = match state.snapshots.find(id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return HttpError::new(
                "infra::http::admin::snapshots::edit",
                StatusCode::NOT_FOUND,
                "Snapshot not found",
                "Snapshot not found".to_string(),
            )
            .into_response();
        }
        Err(err) => {
            return HttpError::new(
                "infra::http::admin::snapshots::edit",
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load snapshot",
                err.to_string(),
            )
            .into_response();
        }
    };

    let entity_slug = entity_slug(record.entity_type);
    let chrome = match state.chrome.load(entity_slug).await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let view = build_editor_view(&record);

    let layout = admin_views::AdminLayout::new(chrome, view);
    render_template_response(
        admin_views::AdminSnapshotEditTemplate { view: layout },
        StatusCode::OK,
    )
}

pub async fn admin_snapshot_update(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<SnapshotEditForm>,
) -> Response {
    let description = blank_to_none_opt(form.description);

    let record = match state.snapshots.update_description(id, description).await {
        Ok(record) => record,
        Err(err) => return map_error(err),
    };

    Redirect::to(&format!("/snapshots/{}/edit", record.id)).into_response()
}

pub(super) fn build_editor_view(
    record: &crate::application::repos::SnapshotRecord,
) -> admin_views::AdminSnapshotEditorView {
    let entity_slug = entity_slug(record.entity_type);
    admin_views::AdminSnapshotEditorView {
        heading: "Edit Snapshot".to_string(),
        entity_label: entity_label(record.entity_type).to_string(),
        form_action: format!("/snapshots/{}/edit", record.id),
        back_href: format!("/{}/{}/snapshots", entity_slug, record.entity_id),
        version: record.version,
        description: record.description.clone(),
        submit_label: "Save Changes".to_string(),
    }
}

fn map_error(err: SnapshotServiceError) -> Response {
    match err {
        SnapshotServiceError::NotFound => HttpError::new(
            "infra::http::admin::snapshots::update",
            StatusCode::NOT_FOUND,
            "Snapshot not found",
            "Snapshot not found".to_string(),
        )
        .into_response(),
        SnapshotServiceError::Snapshot(inner) => HttpError::new(
            "infra::http::admin::snapshots::update",
            StatusCode::BAD_REQUEST,
            "Snapshot validation failed",
            inner.to_string(),
        )
        .into_response(),
        SnapshotServiceError::Repo(repo) => HttpError::new(
            "infra::http::admin::snapshots::update",
            StatusCode::INTERNAL_SERVER_ERROR,
            "Snapshot repository error",
            repo.to_string(),
        )
        .into_response(),
        SnapshotServiceError::App(app) => HttpError::new(
            "infra::http::admin::snapshots::update",
            StatusCode::INTERNAL_SERVER_ERROR,
            "Snapshot error",
            app.to_string(),
        )
        .into_response(),
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
