//! Tag toggle handlers for posts.

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use datastar::prelude::ElementPatchMode;
use std::collections::BTreeSet;
use uuid::Uuid;

use crate::{
    application::error::HttpError,
    infra::http::admin::{
        AdminState,
        selectors::{TAG_PICKER, TAG_SELECTION_STORE},
        shared::{datastar_replace, template_render_http_error},
    },
    presentation::admin::views as admin_views,
};

use super::errors::admin_post_error;
use super::forms::AdminPostTagsToggleForm;
use super::sections::{build_tag_picker_view, load_tag_counts};

pub(crate) async fn admin_post_tags_toggle(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPostTagsToggleForm>,
) -> Response {
    handle_post_tag_toggle(&state, Some(id), form).await
}

pub(crate) async fn admin_post_tags_toggle_new(
    State(state): State<AdminState>,
    Form(form): Form<AdminPostTagsToggleForm>,
) -> Response {
    handle_post_tag_toggle(&state, None, form).await
}

async fn handle_post_tag_toggle(
    state: &AdminState,
    post_id: Option<Uuid>,
    form: AdminPostTagsToggleForm,
) -> Response {
    if let Err(response) = ensure_post_exists_for_tag_actions(state, post_id).await {
        return response;
    }

    let mut selected_ids = parse_tag_state(&form.tag_state);
    if let Some(index) = selected_ids.iter().position(|id| *id == form.tag_id) {
        selected_ids.remove(index);
    } else {
        selected_ids.push(form.tag_id);
    }

    render_tag_picker_response(state, post_id, &selected_ids).await
}

async fn ensure_post_exists_for_tag_actions(
    state: &AdminState,
    post_id: Option<Uuid>,
) -> Result<(), Response> {
    let Some(id) = post_id else {
        return Ok(());
    };

    match state.posts.load_post(id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(HttpError::new(
            "infra::http::admin_post_tags_lookup",
            StatusCode::NOT_FOUND,
            "Post not found",
            format!("Post `{id}` could not be found"),
        )
        .into_response()),
        Err(err) => {
            Err(admin_post_error("infra::http::admin_post_tags_lookup", err).into_response())
        }
    }
}

async fn render_tag_picker_response(
    state: &AdminState,
    post_id: Option<Uuid>,
    selected_ids: &[Uuid],
) -> Response {
    let mut normalized = Vec::new();
    let mut seen = BTreeSet::new();
    for id in selected_ids {
        if seen.insert(*id) {
            normalized.push(*id);
        }
    }

    let tags_with_counts = match load_tag_counts(state).await {
        Ok(tags) => tags,
        Err(err) => return err.into_response(),
    };

    let picker_view = build_tag_picker_view(post_id, &tags_with_counts, &normalized);

    let picker_template = admin_views::AdminPostTagPickerTemplate {
        picker: picker_view.clone(),
    };
    let picker_html = match picker_template.render() {
        Ok(html) => html,
        Err(err) => {
            return template_render_http_error(
                "infra::http::admin_post_tags_render",
                "Template rendering failed",
                err,
            )
            .into_response();
        }
    };

    let store_template = admin_views::AdminPostTagSelectionStoreTemplate {
        picker: picker_view,
    };
    let store_html = match store_template.render() {
        Ok(html) => html,
        Err(err) => {
            return template_render_http_error(
                "infra::http::admin_post_tags_render",
                "Template rendering failed",
                err,
            )
            .into_response();
        }
    };

    let mut stream = datastar_replace(TAG_PICKER, picker_html);
    stream.push_patch(store_html, TAG_SELECTION_STORE, ElementPatchMode::Replace);
    stream.into_response()
}

pub(super) fn parse_tag_state(state: &Option<String>) -> Vec<Uuid> {
    state
        .as_deref()
        .map(|raw| {
            raw.split(',')
                .filter_map(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Uuid::parse_str(trimmed).ok()
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}
