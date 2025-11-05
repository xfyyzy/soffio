use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use datastar::prelude::ElementPatchMode;
use std::collections::BTreeSet;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        admin::posts::{
            AdminPostError, CreatePostCommand, UpdatePostContentCommand, UpdatePostStatusCommand,
        },
        error::HttpError,
        pagination::PostCursor,
        repos::{PostQueryFilter, SettingsRepo},
    },
    domain::{entities::PostRecord, types::PostStatus},
    infra::http::admin::{
        AdminState,
        shared::{
            AdminPostQuery, EditorSuccessRender, Toast, blank_to_none_opt, datastar_replace,
            push_toasts, stream_editor_success, template_render_http_error,
        },
    },
    presentation::{admin::views as admin_views, views::render_template_response},
};

use crate::infra::http::admin::{
    pagination::{self, CursorState},
    selectors::{POSTS_PANEL, TAG_PICKER, TAG_SELECTION_STORE},
};

use super::{
    errors::admin_post_error,
    forms::{
        AdminPostBulkActionForm, AdminPostDeleteForm, AdminPostForm, AdminPostPanelForm,
        AdminPostPinForm, AdminPostStatusActionForm, AdminPostTagsToggleForm,
    },
    panel::{build_post_list_view, build_post_panel_html, render_post_panel_html},
    sections::{
        build_new_post_editor_view, build_post_editor_view, build_tag_picker_view, load_tag_counts,
    },
    status::parse_post_status,
};

enum BulkAction {
    Publish,
    Draft,
    Archive,
    Delete,
}

pub(super) fn apply_pagination_links(
    content: &mut admin_views::AdminPostListView,
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

impl BulkAction {
    fn from_str(action: &str) -> Option<Self> {
        match action {
            "publish" => Some(Self::Publish),
            "draft" => Some(Self::Draft),
            "archive" => Some(Self::Archive),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            BulkAction::Publish => "Publish",
            BulkAction::Draft => "Move to Draft",
            BulkAction::Archive => "Archive",
            BulkAction::Delete => "Delete",
        }
    }
}

fn normalize_filter_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
}

fn build_post_filter(
    search: Option<&str>,
    tag: Option<&str>,
    month: Option<&str>,
) -> PostQueryFilter {
    PostQueryFilter {
        search: normalize_filter_value(search),
        tag: normalize_filter_value(tag),
        month: normalize_filter_value(month),
    }
}

pub(crate) async fn admin_posts(
    State(state): State<AdminState>,
    Query(query): Query<AdminPostQuery>,
) -> Response {
    let chrome = match state.chrome.load("/posts").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(query.cursor.clone(), query.trail.clone());

    let status = match parse_post_status(query.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let cursor = match cursor_state.decode_with(PostCursor::decode, "infra::http::admin_posts") {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = build_post_filter(
        query.search.as_deref(),
        query.tag.as_deref(),
        query.month.as_deref(),
    );

    let mut content = match build_post_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_post_error("infra::http::admin_posts", err).into_response(),
    };

    apply_pagination_links(&mut content, &cursor_state);

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminPostsTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_posts_panel(
    State(state): State<AdminState>,
    Form(form): Form<AdminPostPanelForm>,
) -> Response {
    let status = match parse_post_status(form.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let cursor =
        match cursor_state.decode_with(PostCursor::decode, "infra::http::admin_posts_panel") {
            Ok(cursor) => cursor,
            Err(err) => return err.into_response(),
        };

    let filter = if form.clear.is_some() {
        build_post_filter(None, None, None)
    } else {
        build_post_filter(
            form.search.as_deref(),
            form.tag.as_deref(),
            form.month.as_deref(),
        )
    };

    let mut content = match build_post_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_post_error("infra::http::admin_posts_panel", err).into_response(),
    };

    apply_pagination_links(&mut content, &cursor_state);

    let panel_html = match render_post_panel_html(&content, "infra::http::admin_posts_panel") {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    datastar_replace(POSTS_PANEL, panel_html).into_response()
}

pub(crate) async fn admin_post_new(State(state): State<AdminState>) -> Response {
    let chrome = match state.chrome.load("/posts").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let content = match build_new_post_editor_view(&state).await {
        Ok(view) => view,
        Err(err) => return err.into_response(),
    };
    let view = admin_views::AdminLayout::new(chrome, content);

    render_template_response(admin_views::AdminPostEditTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_post_edit(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
) -> Response {
    let chrome = match state.chrome.load("/posts").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let post = match state.posts.load_post(id).await {
        Ok(Some(post)) => post,
        Ok(None) => {
            return HttpError::new(
                "infra::http::admin_post_edit",
                StatusCode::NOT_FOUND,
                "Post not found",
                format!("Post `{id}` could not be found"),
            )
            .into_response();
        }
        Err(err) => return admin_post_error("infra::http::admin_post_edit", err).into_response(),
    };

    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => {
            return admin_post_error("infra::http::admin_post_edit", err.into()).into_response();
        }
    };

    let content = match build_post_editor_view(&state, &post, timezone).await {
        Ok(view) => view,
        Err(err) => return err.into_response(),
    };

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminPostEditTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_post_update(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPostForm>,
) -> Response {
    let post = match state.posts.load_post(id).await {
        Ok(Some(post)) => post,
        Ok(None) => {
            return HttpError::new(
                "infra::http::admin_post_update",
                StatusCode::NOT_FOUND,
                "Post not found",
                format!("Post `{id}` could not be found"),
            )
            .into_response();
        }
        Err(err) => return admin_post_error("infra::http::admin_post_update", err).into_response(),
    };
    let tag_ids = parse_tag_state(&form.tag_state);

    let status = match parse_post_status(Some(form.status.as_str())) {
        Ok(option) => option.unwrap_or(post.status),
        Err(err) => return err.into_response(),
    };

    let summary_markdown = blank_to_none_opt(form.summary_markdown);
    let pinned = parse_checkbox_flag(&form.pinned);

    let command = UpdatePostContentCommand {
        id: post.id,
        slug: post.slug.clone(),
        title: form.title.trim().to_string(),
        excerpt: form.excerpt.trim().to_string(),
        body_markdown: form.body_markdown.trim().to_string(),
        pinned,
        summary_markdown,
    };

    let actor = "admin";

    let updated = match state.posts.update_post(actor, command).await {
        Ok(post) => post,
        Err(err) => return admin_post_error("infra::http::admin_post_update", err).into_response(),
    };

    let final_record = if updated.status != status {
        let status_command = UpdatePostStatusCommand {
            id: updated.id,
            status,
            scheduled_at: post.scheduled_at,
            published_at: post.published_at,
            archived_at: post.archived_at,
        };
        match state.posts.update_status(actor, status_command).await {
            Ok(post) => post,
            Err(err) => {
                return admin_post_error("infra::http::admin_post_update", err).into_response();
            }
        }
    } else {
        updated
    };

    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => {
            return admin_post_error("infra::http::admin_post_update", err.into()).into_response();
        }
    };

    if let Err(err) = state
        .posts
        .replace_tags(actor, &final_record, &tag_ids)
        .await
    {
        return admin_post_error("infra::http::admin_post_update", err).into_response();
    }

    let content = match build_post_editor_view(&state, &final_record, timezone).await {
        Ok(view) => view,
        Err(err) => return err.into_response(),
    };

    let template = admin_views::AdminPostEditPanelTemplate {
        content: content.clone(),
    };

    let panel_html = match template.render() {
        Ok(html) => html,
        Err(err) => {
            return template_render_http_error(
                "infra::http::admin_post_update",
                "Template rendering failed",
                err,
            )
            .into_response();
        }
    };

    let mut stream = datastar_replace("[data-role=\"panel\"]", panel_html);

    if let Err(err) = push_toasts(
        &mut stream,
        &[Toast::success(format!(
            "Saved post \"{}\"",
            final_record.title
        ))],
    ) {
        return err.into_response();
    }

    stream.push_script(format!(
        "window.history.replaceState(null, '', '/posts/{}/edit');",
        final_record.id
    ));

    stream.into_response()
}

pub(crate) async fn admin_post_create(
    State(state): State<AdminState>,
    Form(form): Form<AdminPostForm>,
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

    let status_value = match parse_post_status(Some(form.status.as_str())) {
        Ok(Some(status)) => status,
        Ok(None) => PostStatus::Draft,
        Err(err) => return err.into_response(),
    };

    let summary_markdown = blank_to_none_opt(form.summary_markdown);
    let title = form.title.trim().to_string();
    let excerpt = form.excerpt.trim().to_string();
    let body_markdown = form.body_markdown.trim().to_string();
    let pinned = parse_checkbox_flag(&form.pinned);

    let tag_ids = parse_tag_state(&form.tag_state);

    let command = CreatePostCommand {
        title: title.clone(),
        excerpt: excerpt.clone(),
        body_markdown: body_markdown.clone(),
        summary_markdown: summary_markdown.clone(),
        status: status_value,
        pinned,
        scheduled_at: None,
        published_at: None,
        archived_at: None,
    };

    let actor = "admin";

    match state.posts.create_post(actor, command).await {
        Ok(post) => {
            let mut toasts = Vec::new();
            toasts.push(Toast::success(format!("Created post \"{}\"", post.title)));

            if let Err(err) = state.posts.replace_tags(actor, &post, &tag_ids).await {
                toasts.push(Toast::error(format!(
                    "Created post \"{}\" but failed to update tags: {}",
                    post.title, err
                )));
            }

            respond_with_post_editor_success(
                &state,
                PostEditorSuccess {
                    post: &post,
                    status_filter,
                    filter: &filter,
                    toasts: &toasts,
                    template_source: "infra::http::admin_post_create",
                },
            )
            .await
        }
        Err(err) => {
            let message = Toast::error(format!("Failed to create post: {}", err));
            respond_with_posts_panel(
                &state,
                status_filter,
                &filter,
                &[message],
                "infra::http::admin_post_create",
                "infra::http::admin_post_create",
            )
            .await
        }
    }
}

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

fn parse_tag_state(state: &Option<String>) -> Vec<Uuid> {
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

fn parse_checkbox_flag(value: &Option<String>) -> bool {
    matches!(value.as_deref(), Some("true") | Some("on") | Some("1"))
}

pub(crate) async fn admin_post_delete(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPostDeleteForm>,
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
                &state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                "infra::http::admin_post_delete",
                "infra::http::admin_post_delete",
            )
            .await;
        }
        Err(err) => {
            let message = Toast::error(format!("Failed to load post: {}", err));
            return respond_with_posts_panel_message(
                &state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                "infra::http::admin_post_delete",
                "infra::http::admin_post_delete",
            )
            .await;
        }
    };

    if let Err(err) = state.posts.delete_post(actor, post.id).await {
        let message = Toast::error(format!("Failed to delete post: {}", err));
        return respond_with_posts_panel_message(
            &state,
            status_filter,
            &filter,
            &cursor_state,
            message,
            "infra::http::admin_post_delete",
            "infra::http::admin_post_delete",
        )
        .await;
    }

    let message = Toast::success(format!("Deleted post \"{}\"", post.title));
    respond_with_posts_panel_message(
        &state,
        status_filter,
        &filter,
        &cursor_state,
        message,
        "infra::http::admin_post_delete",
        "infra::http::admin_post_delete",
    )
    .await
}

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

fn build_status_update_command(
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

async fn respond_with_posts_panel(
    state: &AdminState,
    status_filter: Option<PostStatus>,
    filter: &PostQueryFilter,
    toasts: &[Toast],
    error_source: &'static str,
    template_source: &'static str,
) -> Response {
    let cursor_state = CursorState::default();
    respond_with_posts_panel_with_state(
        state,
        status_filter,
        filter,
        &cursor_state,
        toasts,
        error_source,
        template_source,
    )
    .await
}

async fn respond_with_posts_panel_message(
    state: &AdminState,
    status_filter: Option<PostStatus>,
    filter: &PostQueryFilter,
    cursor_state: &CursorState,
    message: Toast,
    error_source: &'static str,
    template_source: &'static str,
) -> Response {
    let messages = [message];
    respond_with_posts_panel_with_state(
        state,
        status_filter,
        filter,
        cursor_state,
        &messages,
        error_source,
        template_source,
    )
    .await
}

async fn respond_with_posts_panel_with_state(
    state: &AdminState,
    status_filter: Option<PostStatus>,
    filter: &PostQueryFilter,
    cursor_state: &CursorState,
    toasts: &[Toast],
    error_source: &'static str,
    template_source: &'static str,
) -> Response {
    let cursor = match cursor_state.decode_with(PostCursor::decode, error_source) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut content = match build_post_list_view(state, status_filter, filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_post_error(error_source, err).into_response(),
    };

    apply_pagination_links(&mut content, cursor_state);

    match render_post_panel_html(&content, template_source) {
        Ok(html) => {
            let mut stream = datastar_replace(POSTS_PANEL, html);
            if !toasts.is_empty()
                && let Err(err) = push_toasts(&mut stream, toasts)
            {
                return err.into_response();
            }
            stream.into_response()
        }
        Err(err) => err.into_response(),
    }
}

struct PostEditorSuccess<'a> {
    post: &'a PostRecord,
    status_filter: Option<PostStatus>,
    filter: &'a PostQueryFilter,
    toasts: &'a [Toast],
    template_source: &'static str,
}

async fn respond_with_post_editor_success(
    state: &AdminState,
    params: PostEditorSuccess<'_>,
) -> Response {
    let PostEditorSuccess {
        post,
        status_filter,
        filter,
        toasts,
        template_source,
    } = params;

    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => return admin_post_error(template_source, err.into()).into_response(),
    };

    let content = match build_post_editor_view(state, post, timezone).await {
        Ok(view) => view,
        Err(err) => return err.into_response(),
    };

    let editor_template = admin_views::AdminPostEditPanelTemplate {
        content: content.clone(),
    };

    let editor_html = match editor_template.render() {
        Ok(html) => html,
        Err(err) => {
            return template_render_http_error(template_source, "Template rendering failed", err)
                .into_response();
        }
    };

    let panel_html = match build_post_panel_html(
        state,
        status_filter,
        filter,
        template_source,
        template_source,
    )
    .await
    {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    stream_editor_success(EditorSuccessRender {
        editor_html,
        panel_html,
        panel_selector: POSTS_PANEL,
        toasts,
        history_path: Some(format!("/posts/{}/edit", post.id)),
    })
}
