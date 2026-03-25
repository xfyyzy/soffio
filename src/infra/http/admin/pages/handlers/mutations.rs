use super::*;

pub(crate) async fn admin_page_delete(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPageDeleteForm>,
) -> Response {
    let status_filter = match parse_page_status(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter =
        shared::build_page_filter(form.filter_search.as_deref(), form.filter_month.as_deref());

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let actor = "admin";

    let page = match state.pages.find_by_id(id).await {
        Ok(Some(page)) => page,
        Ok(None) => {
            let message = Toast::error("Page not found");
            return shared::respond_with_pages_panel_message(
                &state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                "infra::http::admin_page_delete",
                "infra::http::admin_page_delete",
            )
            .await;
        }
        Err(err) => {
            let message = Toast::error(format!("Failed to load page: {}", err));
            return shared::respond_with_pages_panel_message(
                &state,
                status_filter,
                &filter,
                &cursor_state,
                message,
                "infra::http::admin_page_delete",
                "infra::http::admin_page_delete",
            )
            .await;
        }
    };

    if let Err(err) = state.pages.delete_page(actor, page.id, &page.slug).await {
        let message = Toast::error(format!("Failed to delete page: {}", err));
        return shared::respond_with_pages_panel_message(
            &state,
            status_filter,
            &filter,
            &cursor_state,
            message,
            "infra::http::admin_page_delete",
            "infra::http::admin_page_delete",
        )
        .await;
    }

    let message = Toast::success(format!("Deleted page \"{}\"", page.title));
    shared::respond_with_pages_panel_message(
        &state,
        status_filter,
        &filter,
        &cursor_state,
        message,
        "infra::http::admin_page_delete",
        "infra::http::admin_page_delete",
    )
    .await
}

pub(crate) async fn admin_page_publish(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPageStatusActionForm>,
) -> Response {
    handle_page_status_action(
        &state,
        id,
        form,
        PageStatus::Published,
        "infra::http::admin_page_publish",
        "infra::http::admin_page_publish",
    )
    .await
}

pub(crate) async fn admin_page_move_to_draft(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPageStatusActionForm>,
) -> Response {
    handle_page_status_action(
        &state,
        id,
        form,
        PageStatus::Draft,
        "infra::http::admin_page_move_to_draft",
        "infra::http::admin_page_move_to_draft",
    )
    .await
}

pub(crate) async fn admin_page_archive(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminPageStatusActionForm>,
) -> Response {
    handle_page_status_action(
        &state,
        id,
        form,
        PageStatus::Archived,
        "infra::http::admin_page_archive",
        "infra::http::admin_page_archive",
    )
    .await
}

async fn handle_page_status_action(
    state: &AdminState,
    id: Uuid,
    form: AdminPageStatusActionForm,
    target_status: PageStatus,
    error_source: &'static str,
    template_source: &'static str,
) -> Response {
    let status_filter = match parse_page_status(form.status_filter.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let filter =
        shared::build_page_filter(form.filter_search.as_deref(), form.filter_month.as_deref());
    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let page = match state.pages.find_by_id(id).await {
        Ok(Some(page)) => page,
        Ok(None) => {
            let message = Toast::error("Page not found");
            return shared::respond_with_pages_panel_message(
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
            let message = Toast::error(format!("Failed to load page: {}", err));
            return shared::respond_with_pages_panel_message(
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

    let actor = "admin";
    let command = UpdatePageStatusCommand {
        id: page.id,
        status: target_status,
        scheduled_at: page.scheduled_at,
        published_at: page.published_at,
        archived_at: page.archived_at,
    };

    let message = match state.pages.update_status(actor, command).await {
        Ok(updated) => Toast::success(format!(
            "Updated page \"{}\" to {}",
            updated.title,
            page_status_label(target_status),
        )),
        Err(err) => Toast::error(format!("Failed to update page status: {}", err)),
    };

    shared::respond_with_pages_panel_message(
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
