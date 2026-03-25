use super::shared::IfEmptyExt;
use super::*;

pub(crate) async fn admin_tag_delete(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminTagDeleteForm>,
) -> Response {
    let pinned_filter = match parse_tag_status(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter =
        shared::build_tag_filter(form.filter_search.as_deref(), form.filter_month.as_deref());
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let tag = match state.tags.find_by_id(id).await {
        Ok(Some(tag)) => tag,
        Ok(None) => {
            let message = Toast::error("Tag not found");
            return shared::respond_with_tags_panel_internal(shared::TagPanelResponseParams {
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

    shared::respond_with_tags_panel_internal(shared::TagPanelResponseParams {
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

    let filter =
        shared::build_tag_filter(form.filter_search.as_deref(), form.filter_month.as_deref());
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let actor = "admin";

    let message = match state.tags.update_tag_pinned(actor, id, pinned).await {
        Ok(tag) => {
            let verb = tag_status_label(pinned);
            Toast::success(format!("{verb} tag \"{}\"", tag.name))
        }
        Err(err) => Toast::error(format!("Failed to update tag: {err}")),
    };

    shared::respond_with_tags_panel_internal(shared::TagPanelResponseParams {
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
