use super::*;

pub(crate) async fn admin_navigation_toggle_visibility(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminNavigationVisibilityForm>,
) -> Response {
    let status = match parse_navigation_status(form.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let cursor = match cursor_state.decode_with(
        NavigationCursor::decode,
        "infra::http::admin_navigation_toggle_visibility",
    ) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = build_navigation_filter(form.search.as_deref());

    let item = match state.navigation.find_by_id(id).await {
        Ok(Some(item)) => item,
        Ok(None) => {
            return render_navigation_error_panel(
                &state,
                status,
                &filter,
                cursor,
                &cursor_state,
                "Navigation item not found",
            )
            .await;
        }
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_toggle_visibility", err)
                .into_response();
        }
    };

    let new_visibility = !item.visible;
    let (destination_page_id, destination_url) = match item.destination_type {
        NavigationDestinationType::Internal => {
            if let Some(page_id) = item.destination_page_id {
                (Some(page_id), None)
            } else {
                return render_navigation_error_panel(
                    &state,
                    status,
                    &filter,
                    cursor,
                    &cursor_state,
                    format!(
                        "Navigation item \"{}\" is missing an internal destination",
                        item.label
                    ),
                )
                .await;
            }
        }
        NavigationDestinationType::External => (None, item.destination_url.clone()),
    };

    let command = UpdateNavigationItemCommand {
        id,
        label: item.label.clone(),
        destination_type: item.destination_type,
        destination_page_id,
        destination_url,
        sort_order: item.sort_order,
        visible: new_visibility,
        open_in_new_tab: item.open_in_new_tab,
    };

    let actor = "admin";

    let result = state.navigation.update_item(actor, command).await;

    let mut content = match build_navigation_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_toggle_visibility", err)
                .into_response();
        }
    };

    apply_navigation_pagination_links(&mut content, &cursor_state);

    let panel_html = match render_navigation_panel_html(
        &content,
        "infra::http::admin_navigation_toggle_visibility",
    ) {
        Ok(html) => html,
        Err(err) => return err.into_response(),
    };

    let mut stream = datastar_replace(NAVIGATION_PANEL, panel_html);

    match result {
        Ok(updated) => {
            let message = if updated.visible {
                format!("Marked navigation item \"{}\" as visible", updated.label)
            } else {
                format!("Marked navigation item \"{}\" as hidden", updated.label)
            };
            if let Err(err) = push_toasts(&mut stream, &[Toast::success(message)]) {
                return err.into_response();
            }
        }
        Err(err) => {
            if let Err(push_err) = push_toasts(
                &mut stream,
                &[Toast::error(format!(
                    "Failed to toggle navigation item visibility: {}",
                    err
                ))],
            ) {
                return push_err.into_response();
            }
        }
    }

    stream.into_response()
}

pub(crate) async fn admin_navigation_delete(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Form(form): Form<AdminNavigationDeleteForm>,
) -> Response {
    let status = match parse_navigation_status(form.status.as_deref()) {
        Ok(status) => status,
        Err(err) => return err.into_response(),
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());

    let cursor = match cursor_state.decode_with(
        NavigationCursor::decode,
        "infra::http::admin_navigation_delete",
    ) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let filter = build_navigation_filter(form.search.as_deref());

    let item = match state.navigation.find_by_id(id).await {
        Ok(Some(item)) => item,
        Ok(None) => {
            let panel_html = match build_navigation_panel_html(
                &state,
                status,
                &filter,
                "infra::http::admin_navigation_delete",
                "infra::http::admin_navigation_delete",
            )
            .await
            {
                Ok(html) => html,
                Err(err) => return err.into_response(),
            };

            let mut stream = datastar_replace(NAVIGATION_PANEL, panel_html);
            if let Err(err) = push_toasts(
                &mut stream,
                &[Toast::error("Navigation item not found".to_string())],
            ) {
                return err.into_response();
            }
            return stream.into_response();
        }
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_delete", err)
                .into_response();
        }
    };

    if let Err(err) = state.navigation.delete_item("admin", id).await {
        return admin_navigation_error("infra::http::admin_navigation_delete", err).into_response();
    }

    let mut content = match build_navigation_list_view(&state, status, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_delete", err)
                .into_response();
        }
    };

    apply_navigation_pagination_links(&mut content, &cursor_state);

    let panel_html =
        match render_navigation_panel_html(&content, "infra::http::admin_navigation_delete") {
            Ok(html) => html,
            Err(err) => return err.into_response(),
        };

    let mut stream = datastar_replace(NAVIGATION_PANEL, panel_html);
    if let Err(err) = push_toasts(
        &mut stream,
        &[Toast::success(format!(
            "Deleted navigation item \"{}\"",
            item.label
        ))],
    ) {
        return err.into_response();
    }

    stream.into_response()
}

async fn render_navigation_error_panel(
    state: &AdminState,
    status: NavigationListStatus,
    filter: &NavigationQueryFilter,
    cursor: Option<NavigationCursor>,
    cursor_state: &CursorState,
    message: impl Into<String>,
) -> Response {
    let mut content = match build_navigation_list_view(state, status, filter, cursor).await {
        Ok(content) => content,
        Err(err) => {
            return admin_navigation_error("infra::http::admin_navigation_error_panel", err)
                .into_response();
        }
    };

    apply_navigation_pagination_links(&mut content, cursor_state);

    let panel_html =
        match render_navigation_panel_html(&content, "infra::http::admin_navigation_error_panel") {
            Ok(html) => html,
            Err(err) => return err.into_response(),
        };

    let mut stream = datastar_replace(NAVIGATION_PANEL, panel_html);
    if let Err(err) = push_toasts(&mut stream, &[Toast::error(message.into())]) {
        return err.into_response();
    }

    stream.into_response()
}
