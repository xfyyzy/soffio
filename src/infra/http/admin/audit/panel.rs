//! Panel building for audit list.

use askama::Template;

use crate::{
    application::{
        error::HttpError,
        pagination::{AuditCursor, PageRequest},
        repos::{AuditQueryFilter, SettingsRepo},
    },
    infra::http::admin::{
        AdminState,
        pagination::{self, CursorState},
        shared::template_render_http_error,
    },
    presentation::admin::views as admin_views,
};

use super::status::{action_options, actor_options, entity_type_filters};

/// Build the complete audit list view for rendering.
pub(super) async fn build_audit_list_view(
    state: &AdminState,
    filter: &AuditQueryFilter,
    cursor: Option<AuditCursor>,
) -> Result<admin_views::AdminAuditListView, crate::application::repos::RepoError> {
    let settings = state.db.load_site_settings().await?;
    let admin_page_size = settings.admin_page_size.clamp(1, 100).max(1) as u32;

    // Build filters for counts (without entity_type to get all entity types)
    let count_filter = AuditQueryFilter {
        actor: filter.actor.clone(),
        action: filter.action.clone(),
        entity_type: None, // Don't filter for entity type counts
        search: filter.search.clone(),
    };

    // Parallel queries
    let page_request = PageRequest::new(admin_page_size, cursor);
    let (page, total_count, entity_type_counts, actor_counts, action_counts) = tokio::try_join!(
        state.audit.list_filtered(page_request, filter),
        state.audit.count_filtered(&count_filter),
        state.audit.entity_type_counts(&count_filter),
        state.audit.actor_counts(filter),
        state.audit.action_counts(filter),
    )?;

    let entries: Vec<admin_views::AdminAuditRowView> = page
        .items
        .into_iter()
        .map(|entry| admin_views::AdminAuditRowView {
            id: entry.id.to_string(),
            detail_href: format!("/audit/{}", entry.id),
            actor: entry.actor,
            action: entry.action,
            entity_type: entry.entity_type,
            entity_id: entry.entity_id,
            payload_text: entry.payload_text,
            created_at: admin_views::format_timestamp(entry.created_at, settings.timezone),
        })
        .collect();

    let filters = entity_type_filters(
        &entity_type_counts,
        total_count,
        filter.entity_type.as_deref(),
    );
    let actor_opts = actor_options(&actor_counts, filter.actor.as_deref());
    let action_opts = action_options(&action_counts, filter.action.as_deref());

    Ok(admin_views::AdminAuditListView {
        heading: "Audit Log".to_string(),
        filters,
        entries,
        actor_options: actor_opts,
        action_options: action_opts,
        filter_actor: filter.actor.clone(),
        filter_action: filter.action.clone(),
        filter_entity_type: filter.entity_type.clone(),
        filter_search: filter.search.clone(),
        filter_query: String::new(),
        active_status_key: filter.entity_type.clone(),
        next_cursor: page.next_cursor,
        cursor_param: None,
        trail: None,
        previous_page_state: None,
        next_page_state: None,
        panel_action: "/audit/panel".to_string(),
        custom_hidden_fields: build_audit_hidden_fields(filter),
    })
}

fn build_audit_hidden_fields(filter: &AuditQueryFilter) -> Vec<admin_views::AdminHiddenField> {
    let mut fields = Vec::new();
    if let Some(ref actor) = filter.actor {
        fields.push(admin_views::AdminHiddenField::new("actor", actor.clone()));
    }
    if let Some(ref action) = filter.action {
        fields.push(admin_views::AdminHiddenField::new("action", action.clone()));
    }
    fields
}

pub(super) fn apply_pagination_links(
    content: &mut admin_views::AdminAuditListView,
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

pub(super) fn render_audit_panel_html(
    content: &admin_views::AdminAuditListView,
    template_source: &'static str,
) -> Result<String, HttpError> {
    let template = admin_views::AdminAuditPanelTemplate {
        content: content.clone(),
    };

    template.render().map_err(|err| {
        template_render_http_error(template_source, "Template rendering failed", err)
    })
}
