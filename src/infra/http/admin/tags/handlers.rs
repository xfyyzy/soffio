use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use uuid::Uuid;

use crate::{
    application::{
        admin::tags::{CreateTagCommand, UpdateTagCommand},
        pagination::TagCursor,
        repos::TagQueryFilter,
    },
    domain::entities::TagRecord,
    infra::http::admin::{
        AdminState,
        selectors::{PANEL, TAGS_PANEL},
        shared::{
            AdminPostQuery, EditorSuccessRender, Toast, blank_to_none_opt, datastar_replace,
            push_toasts, stream_editor_success, template_render_http_error,
        },
    },
    presentation::{admin::views as admin_views, views::render_template_response},
};

use crate::infra::http::admin::pagination::CursorState;

use super::{
    editor::{build_new_tag_view, build_tag_edit_view},
    errors::admin_tag_error,
    forms::{AdminTagDeleteForm, AdminTagForm, AdminTagPanelForm, AdminTagPinForm},
    panel::{apply_pagination_links, build_tag_list_view, render_tag_panel_html},
    status::{parse_tag_status, tag_status_label},
};

fn normalize_filter_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
}

fn build_tag_filter(search: Option<&str>, month: Option<&str>) -> TagQueryFilter {
    TagQueryFilter {
        search: normalize_filter_value(search),
        month: normalize_filter_value(month),
    }
}

fn parse_checkbox_flag(input: &Option<String>) -> bool {
    matches!(input.as_deref(), Some("on") | Some("true"))
}

pub(crate) async fn admin_tags(
    State(state): State<AdminState>,
    Query(query): Query<AdminPostQuery>,
) -> Response {
    let chrome = match state.chrome.load("/tags").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(query.cursor.clone(), query.trail.clone());

    let pinned_filter = match parse_tag_status(query.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter = build_tag_filter(query.search.as_deref(), query.month.as_deref());

    let cursor = match cursor_state.decode_with(TagCursor::decode, "infra::http::admin_tags") {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut content = match build_tag_list_view(&state, pinned_filter, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_tag_error("infra::http::admin_tags", err).into_response(),
    };

    apply_pagination_links(&mut content, &cursor_state);

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminTagsTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_tags_panel(
    State(state): State<AdminState>,
    Form(form): Form<AdminTagPanelForm>,
) -> Response {
    let pinned_filter = match parse_tag_status(form.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let cursor = match cursor_state.decode_with(TagCursor::decode, "infra::http::admin_tags_panel")
    {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = if form.clear.is_some() {
        build_tag_filter(None, None)
    } else {
        build_tag_filter(form.search.as_deref(), form.month.as_deref())
    };

    let mut content = match build_tag_list_view(&state, pinned_filter, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_tag_error("infra::http::admin_tags_panel", err).into_response(),
    };

    apply_pagination_links(&mut content, &cursor_state);

    let panel_html = match render_tag_panel_html(&content, "infra::http::admin_tags_panel") {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    datastar_replace(TAGS_PANEL, panel_html).into_response()
}

pub(crate) async fn admin_tag_new(State(state): State<AdminState>) -> Response {
    let chrome = match state.chrome.load("/tags").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let content = build_new_tag_view();
    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminTagEditTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_tag_edit(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
) -> Response {
    let chrome = match state.chrome.load("/tags").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let tag = match state.tags.find_by_id(id).await {
        Ok(Some(tag)) => tag,
        Ok(None) => return Redirect::to("/tags").into_response(),
        Err(err) => return admin_tag_error("infra::http::admin_tag_edit", err).into_response(),
    };

    let content = build_tag_edit_view(&tag);

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminTagEditTemplate { view }, StatusCode::OK)
}

pub(crate) async fn admin_tag_update(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminTagForm>,
) -> Response {
    let actor = "admin";

    let command = UpdateTagCommand {
        id,
        name: form.name.trim().to_string(),
        description: blank_to_none_opt(form.description),
        pinned: parse_checkbox_flag(&form.pinned),
    };

    let updated = match state.tags.update_tag(actor, command).await {
        Ok(tag) => tag,
        Err(err) => return admin_tag_error("infra::http::admin_tag_update", err).into_response(),
    };

    let content = build_tag_edit_view(&updated);

    let template = admin_views::AdminTagEditPanelTemplate {
        content: content.clone(),
    };

    let panel_html = match template.render() {
        Ok(html) => html,
        Err(err) => {
            return template_render_http_error(
                "infra::http::admin_tag_update",
                "Template rendering failed",
                err,
            )
            .into_response();
        }
    };

    let mut stream = datastar_replace(PANEL, panel_html);
    if let Err(err) = push_toasts(
        &mut stream,
        &[Toast::success(format!("Saved tag \"{}\"", updated.name))],
    ) {
        return err.into_response();
    }

    stream.into_response()
}

pub(crate) async fn admin_tag_create(
    State(state): State<AdminState>,
    Form(form): Form<AdminTagForm>,
) -> Response {
    let pinned_filter = match parse_tag_status(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter = build_tag_filter(form.filter_search.as_deref(), form.filter_month.as_deref());
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let actor = "admin";
    let pinned = parse_checkbox_flag(&form.pinned);

    let command = CreateTagCommand {
        name: form.name.trim().to_string(),
        description: blank_to_none_opt(form.description.clone()),
        pinned,
    };

    match state.tags.create_tag(actor, command).await {
        Ok(tag) => {
            let toasts = [Toast::success(format!("Created tag \"{}\"", tag.name))];
            respond_with_tag_editor_success(
                &state,
                TagEditorSuccess {
                    tag: &tag,
                    pinned_filter,
                    filter: &filter,
                    cursor_state: cursor_state.clone(),
                    toasts: &toasts,
                    template_source: "infra::http::admin_tag_create",
                },
            )
            .await
        }
        Err(err) => {
            let message = Toast::error(format!("Failed to create tag: {}", err));
            respond_with_tags_panel_internal(TagPanelResponseParams {
                state: &state,
                pinned_filter,
                filter: &filter,
                cursor_state,
                toasts: &[message],
                error_source: "infra::http::admin_tag_create",
                template_source: "infra::http::admin_tag_create",
            })
            .await
        }
    }
}

pub(crate) async fn admin_tag_delete(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminTagDeleteForm>,
) -> Response {
    let pinned_filter = match parse_tag_status(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter = build_tag_filter(form.filter_search.as_deref(), form.filter_month.as_deref());
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let tag = match state.tags.find_by_id(id).await {
        Ok(Some(tag)) => tag,
        Ok(None) => {
            let message = Toast::error("Tag not found");
            return respond_with_tags_panel_internal(TagPanelResponseParams {
                state: &state,
                pinned_filter,
                filter: &filter,
                cursor_state: cursor_state.clone(),
                toasts: &[message],
                error_source: "infra::http::admin_tag_delete",
                template_source: "infra::http::admin_tag_delete",
            })
            .await;
        }
        Err(err) => return admin_tag_error("infra::http::admin_tag_delete", err).into_response(),
    };

    let actor = "admin";

    let message = match state.tags.delete_tag(actor, id).await {
        Ok(()) => Toast::success(format!(
            "Deleted tag \"{}\"",
            form.name.trim().if_empty_then(|| tag.name.clone())
        )),
        Err(err) => match err {
            crate::application::admin::tags::AdminTagError::InUse { count } => {
                Toast::error(format!(
                    "Cannot delete tag \"{}\": referenced by {} posts",
                    tag.name, count
                ))
            }
            other => Toast::error(format!("Failed to delete tag: {}", other)),
        },
    };

    respond_with_tags_panel_internal(TagPanelResponseParams {
        state: &state,
        pinned_filter,
        filter: &filter,
        cursor_state,
        toasts: &[message],
        error_source: "infra::http::admin_tag_delete",
        template_source: "infra::http::admin_tag_delete",
    })
    .await
}

pub(crate) async fn admin_tag_pin(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminTagPinForm>,
) -> Response {
    handle_tag_pin_action(
        &state,
        id,
        form,
        true,
        "infra::http::admin_tag_pin",
        "infra::http::admin_tag_pin",
    )
    .await
}

pub(crate) async fn admin_tag_unpin(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminTagPinForm>,
) -> Response {
    handle_tag_pin_action(
        &state,
        id,
        form,
        false,
        "infra::http::admin_tag_unpin",
        "infra::http::admin_tag_unpin",
    )
    .await
}

async fn handle_tag_pin_action(
    state: &AdminState,
    id: Uuid,
    form: AdminTagPinForm,
    pinned: bool,
    error_source: &'static str,
    template_source: &'static str,
) -> Response {
    let pinned_filter = match parse_tag_status(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter = build_tag_filter(form.filter_search.as_deref(), form.filter_month.as_deref());
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let actor = "admin";

    let message = match state.tags.update_tag_pinned(actor, id, pinned).await {
        Ok(tag) => {
            let verb = tag_status_label(pinned);
            Toast::success(format!("{verb} tag \"{}\"", tag.name))
        }
        Err(err) => Toast::error(format!("Failed to update tag: {err}")),
    };

    respond_with_tags_panel_internal(TagPanelResponseParams {
        state,
        pinned_filter,
        filter: &filter,
        cursor_state,
        toasts: &[message],
        error_source,
        template_source,
    })
    .await
}

struct TagPanelResponseParams<'a, 'b> {
    state: &'a AdminState,
    pinned_filter: Option<bool>,
    filter: &'a TagQueryFilter,
    cursor_state: CursorState,
    toasts: &'b [Toast],
    error_source: &'static str,
    template_source: &'static str,
}

struct TagEditorSuccess<'a> {
    tag: &'a TagRecord,
    pinned_filter: Option<bool>,
    filter: &'a TagQueryFilter,
    cursor_state: CursorState,
    toasts: &'a [Toast],
    template_source: &'static str,
}

async fn respond_with_tag_editor_success(
    state: &AdminState,
    params: TagEditorSuccess<'_>,
) -> Response {
    let TagEditorSuccess {
        tag,
        pinned_filter,
        filter,
        cursor_state,
        toasts,
        template_source,
    } = params;

    let editor_view = build_tag_edit_view(tag);

    let editor_template = admin_views::AdminTagEditPanelTemplate {
        content: editor_view.clone(),
    };

    let editor_html = match editor_template.render() {
        Ok(html) => html,
        Err(err) => {
            return template_render_http_error(template_source, "Template rendering failed", err)
                .into_response();
        }
    };

    let cursor = match cursor_state.decode_with(TagCursor::decode, template_source) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut list_content = match build_tag_list_view(state, pinned_filter, filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_tag_error(template_source, err).into_response(),
    };

    apply_pagination_links(&mut list_content, &cursor_state);

    let panel_html = match render_tag_panel_html(&list_content, template_source) {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    stream_editor_success(EditorSuccessRender {
        editor_html,
        panel_html,
        panel_selector: TAGS_PANEL,
        toasts,
        history_path: Some(format!("/tags/{}/edit", tag.id)),
    })
}

async fn respond_with_tags_panel_internal(params: TagPanelResponseParams<'_, '_>) -> Response {
    let TagPanelResponseParams {
        state,
        pinned_filter,
        filter,
        cursor_state,
        toasts,
        error_source,
        template_source,
    } = params;

    let cursor = match cursor_state.decode_with(TagCursor::decode, error_source) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut content = match build_tag_list_view(state, pinned_filter, filter, cursor).await {
        Ok(content) => content,
        Err(err) => return admin_tag_error(error_source, err).into_response(),
    };

    apply_pagination_links(&mut content, &cursor_state);

    match render_tag_panel_html(&content, template_source) {
        Ok(html) => {
            let mut stream = datastar_replace(TAGS_PANEL, html);
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

trait IfEmptyExt {
    fn if_empty_then<F: FnOnce() -> String>(self, default: F) -> String;
}

impl IfEmptyExt for &str {
    fn if_empty_then<F: FnOnce() -> String>(self, default: F) -> String {
        if self.trim().is_empty() {
            default()
        } else {
            self.to_string()
        }
    }
}
