//! CRUD handlers for posts - list, create, edit, update, delete.

use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{
    application::{
        admin::posts::{CreatePostCommand, UpdatePostContentCommand, UpdatePostStatusCommand},
        error::HttpError,
        pagination::PostCursor,
        repos::SettingsRepo,
    },
    domain::types::PostStatus,
    infra::http::admin::{
        AdminState,
        pagination::CursorState,
        selectors::POSTS_PANEL,
        shared::{
            Toast, blank_to_none_opt, datastar_replace, push_toasts, template_render_http_error,
        },
    },
    presentation::{admin::views as admin_views, views::render_template_response},
};

use super::errors::admin_post_error;
use super::forms::{AdminPostDeleteForm, AdminPostForm, AdminPostPanelForm};
use super::pagination::apply_pagination_links;
use super::panel::{build_post_list_view, render_post_panel_html};
use super::response::{
    PostEditorSuccess, respond_with_post_editor_success, respond_with_posts_panel,
    respond_with_posts_panel_message,
};
use super::sections::{build_new_post_editor_view, build_post_editor_view};
use super::status::parse_post_status;
use super::tags::parse_tag_state;
use super::utils::{build_post_filter, parse_checkbox_flag};
use crate::infra::http::admin::shared::AdminPostQuery;

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

    if let Err(err) = state.posts.delete_post(actor, post.id, &post.slug).await {
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
