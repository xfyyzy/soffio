//! Bulk action handlers for posts.

use axum::{
    extract::{Form, State},
    response::{IntoResponse, Response},
};
use time::OffsetDateTime;

use crate::{
    application::admin::posts::UpdatePostStatusCommand, domain::types::PostStatus,
    infra::http::admin::AdminState, infra::http::admin::shared::Toast,
};

use super::forms::AdminPostBulkActionForm;
use super::response::respond_with_posts_panel;
use super::status::parse_post_status;
use super::utils::build_post_filter;

#[derive(Clone, Copy)]
pub(super) enum BulkAction {
    Publish,
    Draft,
    Archive,
    Delete,
}

impl BulkAction {
    pub(super) fn from_str(action: &str) -> Option<Self> {
        match action {
            "publish" => Some(Self::Publish),
            "draft" => Some(Self::Draft),
            "archive" => Some(Self::Archive),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            BulkAction::Publish => "Publish",
            BulkAction::Draft => "Move to Draft",
            BulkAction::Archive => "Archive",
            BulkAction::Delete => "Delete",
        }
    }
}

pub(crate) async fn admin_posts_bulk_action(
    State(state): State<AdminState>,
    Form(form): Form<AdminPostBulkActionForm>,
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

    let Some(action) = BulkAction::from_str(form.action.trim()) else {
        let messages = [Toast::error("Select a valid bulk action")];
        return respond_with_posts_panel(
            &state,
            status_filter,
            &filter,
            &messages,
            "infra::http::admin_posts_bulk_action",
            "infra::http::admin_posts_bulk_action",
        )
        .await;
    };

    if form.ids.is_empty() {
        let messages = [Toast::error("Select at least one post")];
        return respond_with_posts_panel(
            &state,
            status_filter,
            &filter,
            &messages,
            "infra::http::admin_posts_bulk_action",
            "infra::http::admin_posts_bulk_action",
        )
        .await;
    }

    let actor = "admin";
    let mut successes = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for id in form.ids.iter().copied() {
        let post = match state.posts.load_post(id).await {
            Ok(Some(post)) => post,
            Ok(None) => {
                failures.push(format!("Post `{id}` not found"));
                continue;
            }
            Err(err) => {
                failures.push(format!("Failed to load `{id}`: {err}"));
                continue;
            }
        };

        let result = match action {
            BulkAction::Delete => state.posts.delete_post(actor, post.id).await.map(|_| ()),
            BulkAction::Publish => {
                let command = UpdatePostStatusCommand {
                    id: post.id,
                    status: PostStatus::Published,
                    scheduled_at: post.scheduled_at,
                    published_at: Some(OffsetDateTime::now_utc()),
                    archived_at: post.archived_at,
                };
                state.posts.update_status(actor, command).await.map(|_| ())
            }
            BulkAction::Draft => {
                let command = UpdatePostStatusCommand {
                    id: post.id,
                    status: PostStatus::Draft,
                    scheduled_at: None,
                    published_at: None,
                    archived_at: None,
                };
                state.posts.update_status(actor, command).await.map(|_| ())
            }
            BulkAction::Archive => {
                let command = UpdatePostStatusCommand {
                    id: post.id,
                    status: PostStatus::Archived,
                    scheduled_at: post.scheduled_at,
                    published_at: post.published_at,
                    archived_at: Some(OffsetDateTime::now_utc()),
                };
                state.posts.update_status(actor, command).await.map(|_| ())
            }
        };

        match result {
            Ok(_) => successes += 1,
            Err(err) => failures.push(format!("{} ({})", post.title, err)),
        }
    }

    let message = if failures.is_empty() {
        Toast::success(format!(
            "{} applied to {} post{}",
            action.label(),
            successes,
            if successes == 1 { "" } else { "s" }
        ))
    } else {
        let sample = failures
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown error".to_string());
        Toast::error(format!(
            "{} succeeded, {} failed (e.g. {})",
            successes,
            failures.len(),
            sample
        ))
    };

    let messages = [message];

    respond_with_posts_panel(
        &state,
        status_filter,
        &filter,
        &messages,
        "infra::http::admin_posts_bulk_action",
        "infra::http::admin_posts_bulk_action",
    )
    .await
}
