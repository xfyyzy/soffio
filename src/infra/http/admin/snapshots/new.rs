use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use uuid::Uuid;

use crate::{
    application::{
        admin::{
            snapshot_types::{PageSnapshotSource, PostSnapshotSource},
            snapshots::SnapshotServiceError,
        },
        error::HttpError,
    },
    infra::http::admin::{AdminState, shared::blank_to_none_opt},
    presentation::{admin::views as admin_views, views::render_template_response},
};

#[derive(Debug, serde::Deserialize)]
pub struct SnapshotCreateForm {
    pub description: Option<String>,
}

pub async fn admin_post_snapshot_new(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
) -> Response {
    render_form(&state, Entity::Post, id).await
}

pub async fn admin_page_snapshot_new(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
) -> Response {
    render_form(&state, Entity::Page, id).await
}

pub async fn admin_post_snapshot_create(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<SnapshotCreateForm>,
) -> Response {
    create_snapshot(&state, Entity::Post, id, form).await
}

pub async fn admin_page_snapshot_create(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<SnapshotCreateForm>,
) -> Response {
    create_snapshot(&state, Entity::Page, id, form).await
}

#[derive(Clone, Copy)]
enum Entity {
    Post,
    Page,
}

impl Entity {
    fn label(self) -> &'static str {
        match self {
            Entity::Post => "Post",
            Entity::Page => "Page",
        }
    }
    fn slug(self) -> &'static str {
        match self {
            Entity::Post => "posts",
            Entity::Page => "pages",
        }
    }
    fn chrome_path(self) -> &'static str {
        self.slug()
    }
}

async fn render_form(state: &AdminState, entity: Entity, id: Uuid) -> Response {
    let chrome = match state.chrome.load(entity.chrome_path()).await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let view = admin_views::AdminSnapshotNewView {
        heading: format!("New {} Snapshot", entity.label()),
        entity_label: entity.label().to_string(),
        form_action: format!("/{}/{}/snapshots/new", entity.slug(), id),
        back_href: format!("/{}/{}/snapshots", entity.slug(), id),
        description: None,
        submit_label: "Create Snapshot".to_string(),
    };

    let layout = admin_views::AdminLayout::new(chrome, view);
    render_template_response(
        admin_views::AdminSnapshotNewTemplate { view: layout },
        StatusCode::OK,
    )
}

async fn create_snapshot(
    state: &AdminState,
    entity: Entity,
    id: Uuid,
    form: SnapshotCreateForm,
) -> Response {
    let description = blank_to_none_opt(form.description);
    let actor = "admin:snapshots";

    let result = match entity {
        Entity::Post => match state.posts.snapshot_source(id).await {
            Ok(source) => {
                state
                    .snapshots
                    .create::<PostSnapshotSource>(actor, &source, description)
                    .await
            }
            Err(err) => {
                return HttpError::new(
                    "infra::http::admin::snapshots::create",
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to load post",
                    err.to_string(),
                )
                .into_response();
            }
        },
        Entity::Page => match state.pages.snapshot_source(id).await {
            Ok(source) => {
                state
                    .snapshots
                    .create::<PageSnapshotSource>(actor, &source, description)
                    .await
            }
            Err(err) => {
                return HttpError::new(
                    "infra::http::admin::snapshots::create",
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to load page",
                    err.to_string(),
                )
                .into_response();
            }
        },
    };

    match result {
        Ok(record) => Redirect::to(&format!("/snapshots/{}/edit", record.id)).into_response(),
        Err(err) => map_error(err),
    }
}

fn map_error(err: SnapshotServiceError) -> Response {
    use SnapshotServiceError::*;
    match err {
        Repo(repo) => HttpError::new(
            "infra::http::admin::snapshots::create",
            StatusCode::INTERNAL_SERVER_ERROR,
            "Snapshot repository error",
            repo.to_string(),
        )
        .into_response(),
        Snapshot(inner) => HttpError::new(
            "infra::http::admin::snapshots::create",
            StatusCode::BAD_REQUEST,
            "Snapshot validation failed",
            inner.to_string(),
        )
        .into_response(),
        App(app) => HttpError::new(
            "infra::http::admin::snapshots::create",
            StatusCode::INTERNAL_SERVER_ERROR,
            "Snapshot error",
            app.to_string(),
        )
        .into_response(),
        NotFound => HttpError::new(
            "infra::http::admin::snapshots::create",
            StatusCode::NOT_FOUND,
            "Snapshot not found",
            "Snapshot not found".to_string(),
        )
        .into_response(),
    }
}
