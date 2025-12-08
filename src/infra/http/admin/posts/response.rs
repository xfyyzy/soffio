//! Response helper functions for post handlers.

use askama::Template;

use crate::{
    application::{
        pagination::PostCursor,
        repos::{PostQueryFilter, SettingsRepo},
    },
    domain::{entities::PostRecord, types::PostStatus},
    infra::http::admin::{
        AdminState,
        pagination::CursorState,
        selectors::POSTS_PANEL,
        shared::{
            EditorSuccessRender, Toast, datastar_replace, push_toasts, stream_editor_success,
            template_render_http_error,
        },
    },
    presentation::admin::views as admin_views,
};
use axum::response::{IntoResponse, Response};

use super::errors::admin_post_error;
use super::pagination::apply_pagination_links;
use super::panel::{build_post_list_view, build_post_panel_html, render_post_panel_html};
use super::sections::build_post_editor_view;

pub(super) async fn respond_with_posts_panel(
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

pub(super) async fn respond_with_posts_panel_message(
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

pub(super) async fn respond_with_posts_panel_with_state(
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

pub(super) struct PostEditorSuccess<'a> {
    pub(super) post: &'a PostRecord,
    pub(super) status_filter: Option<PostStatus>,
    pub(super) filter: &'a PostQueryFilter,
    pub(super) toasts: &'a [Toast],
    pub(super) template_source: &'static str,
}

pub(super) async fn respond_with_post_editor_success(
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
