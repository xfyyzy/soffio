//! Status action handlers for posts (publish, draft, archive).

use axum::{
    extract::{Form, Path, State},
    response::{IntoResponse, Response},
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::admin::posts::{AdminPostError, UpdatePostStatusCommand},
    domain::{entities::PostRecord, types::PostStatus},
    infra::http::admin::{AdminState, pagination::CursorState, shared::Toast},
};

use super::forms::AdminPostStatusActionForm;
use super::response::respond_with_posts_panel_message;
use super::status::parse_post_status;
use super::utils::build_post_filter;

pub(crate) async fn admin_post_publish(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPostStatusActionForm>,
) -> Response {
    handle_post_status_action(
        &state,
        id,
        form,
        PostStatus::Published,
        "infra::http::admin_post_publish",
        "infra::http::admin_post_publish",
    )
    .await
}

pub(crate) async fn admin_post_move_to_draft(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPostStatusActionForm>,
) -> Response {
    handle_post_status_action(
        &state,
        id,
        form,
        PostStatus::Draft,
        "infra::http::admin_post_move_to_draft",
        "infra::http::admin_post_move_to_draft",
    )
    .await
}

pub(crate) async fn admin_post_archive(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPostStatusActionForm>,
) -> Response {
    handle_post_status_action(
        &state,
        id,
        form,
        PostStatus::Archived,
        "infra::http::admin_post_archive",
        "infra::http::admin_post_archive",
    )
    .await
}

async fn handle_post_status_action(
    state: &AdminState,
    id: Uuid,
    form: AdminPostStatusActionForm,
    target_status: PostStatus,
    error_source: &'static str,
    template_source: &'static str,
) -> Response {
    let status_filter = match parse_post_status(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter = build_post_filter(
        form.filter_search.as_deref(),
        form.filter_tag.as_deref(),
        form.filter_month.as_deref(),
    );

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let actor = "admin";

    let post = match state.posts.load_post(id).await {
        Ok(Some(post)) => post,
        Ok(None) => {
            let message = Toast::error("Post not found");
            return respond_with_posts_panel_message(
                state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                error_source,
                template_source,
            )
            .await;
        }
        Err(err) => {
            let message = Toast::error(format!("Failed to load post: {}", err));
            return respond_with_posts_panel_message(
                state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                error_source,
                template_source,
            )
            .await;
        }
    };

    let already_target = post.status == target_status;

    let update_result = if already_target {
        Ok(())
    } else {
        let command = build_status_update_command(&post, target_status);
        state.posts.update_status(actor, command).await.map(|_| ())
    };

    match update_result {
        Ok(()) => {
            let message = Toast::success(status_success_message(
                target_status,
                &post.title,
                already_target,
            ));
            respond_with_posts_panel_message(
                state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                error_source,
                template_source,
            )
            .await
        }
        Err(err) => {
            let message = Toast::error(status_error_message(target_status, &post.title, &err));

            respond_with_posts_panel_message(
                state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                error_source,
                template_source,
            )
            .await
        }
    }
}

pub(super) fn build_status_update_command(
    post: &PostRecord,
    target_status: PostStatus,
) -> UpdatePostStatusCommand {
    match target_status {
        PostStatus::Published => UpdatePostStatusCommand {
            id: post.id,
            status: PostStatus::Published,
            scheduled_at: post.scheduled_at,
            published_at: Some(OffsetDateTime::now_utc()),
            archived_at: post.archived_at,
        },
        PostStatus::Draft => UpdatePostStatusCommand {
            id: post.id,
            status: PostStatus::Draft,
            scheduled_at: None,
            published_at: None,
            archived_at: None,
        },
        PostStatus::Archived => UpdatePostStatusCommand {
            id: post.id,
            status: PostStatus::Archived,
            scheduled_at: post.scheduled_at,
            published_at: post.published_at,
            archived_at: Some(OffsetDateTime::now_utc()),
        },
        other => UpdatePostStatusCommand {
            id: post.id,
            status: other,
            scheduled_at: post.scheduled_at,
            published_at: post.published_at,
            archived_at: post.archived_at,
        },
    }
}

fn status_success_message(target_status: PostStatus, title: &str, already: bool) -> String {
    match target_status {
        PostStatus::Published => {
            if already {
                format!("Post \"{}\" is already published", title)
            } else {
                format!("Published post \"{}\"", title)
            }
        }
        PostStatus::Draft => {
            if already {
                format!("Post \"{}\" is already a draft", title)
            } else {
                format!("Moved post \"{}\" to Draft", title)
            }
        }
        PostStatus::Archived => {
            if already {
                format!("Post \"{}\" is already archived", title)
            } else {
                format!("Archived post \"{}\"", title)
            }
        }
        _ => format!("Updated post \"{}\"", title),
    }
}

fn status_error_message(target_status: PostStatus, title: &str, err: &AdminPostError) -> String {
    let action = match target_status {
        PostStatus::Published => "publish post",
        PostStatus::Draft => "move post to Draft",
        PostStatus::Archived => "archive post",
        _ => "update post",
    };

    format!("Failed to {} \"{}\": {}", action, title, err)
}
