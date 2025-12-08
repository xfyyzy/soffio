//! Pin action handlers for posts.

use axum::{
    extract::{Form, Path, State},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::infra::http::admin::{AdminState, pagination::CursorState, shared::Toast};

use super::forms::AdminPostPinForm;
use super::response::respond_with_posts_panel_message;
use super::status::parse_post_status;
use super::utils::build_post_filter;

pub(crate) async fn admin_post_pin(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPostPinForm>,
) -> Response {
    handle_post_pin_action(
        &state,
        id,
        form,
        true,
        "infra::http::admin_post_pin",
        "infra::http::admin_post_pin",
    )
    .await
}

pub(crate) async fn admin_post_unpin(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPostPinForm>,
) -> Response {
    handle_post_pin_action(
        &state,
        id,
        form,
        false,
        "infra::http::admin_post_unpin",
        "infra::http::admin_post_unpin",
    )
    .await
}

async fn handle_post_pin_action(
    state: &AdminState,
    id: Uuid,
    form: AdminPostPinForm,
    should_pin: bool,
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

    let message = if post.pinned == should_pin {
        let verb = if should_pin { "Pinned" } else { "Unpinned" };
        Toast::success(format!("{} post \"{}\"", verb, post.title))
    } else {
        match state
            .posts
            .update_pin_state(actor, post.id, should_pin)
            .await
        {
            Ok(updated) => {
                let verb = if updated.pinned { "Pinned" } else { "Unpinned" };
                Toast::success(format!("{} post \"{}\"", verb, updated.title))
            }
            Err(err) => Toast::error(format!("Failed to update pin state: {}", err)),
        }
    };

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
