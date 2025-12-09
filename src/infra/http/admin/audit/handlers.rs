//! HTTP handlers for audit admin - list view.

use askama::Template;
use axum::{
    extract::{Form, State},
    response::{IntoResponse, Response},
};

use crate::{
    application::{pagination::AuditCursor, repos::AuditQueryFilter},
    infra::http::admin::{
        AdminState, pagination::CursorState, selectors::AUDIT_PANEL, shared::datastar_replace,
    },
    presentation::admin::views as admin_views,
};

use super::{
    errors::admin_audit_error,
    forms::AdminAuditPanelForm,
    panel::{apply_pagination_links, build_audit_list_view, render_audit_panel_html},
};

/// GET /audit - Render audit log list page.
pub(crate) async fn admin_audit(State(state): State<AdminState>) -> Response {
    let filter = AuditQueryFilter::default();

    let mut content = match build_audit_list_view(&state, &filter, None).await {
        Ok(content) => content,
        Err(err) => {
            return admin_audit_error("infra::http::admin::audit::admin_audit", err)
                .into_response();
        }
    };

    let cursor_state = CursorState::default();
    apply_pagination_links(&mut content, &cursor_state);

    let chrome = match state.chrome.load("/audit").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };
    let view = admin_views::AdminLayout::new(chrome, content);
    let template = admin_views::AdminAuditTemplate { view };

    match template.render() {
        Ok(html) => axum::response::Html(html).into_response(),
        Err(err) => {
            tracing::error!(error = %err, "Failed to render audit template");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// POST /audit/panel - AJAX panel refresh via datastar SSE.
pub(crate) async fn admin_audit_panel(
    State(state): State<AdminState>,
    Form(form): Form<AdminAuditPanelForm>,
) -> Response {
    let (actor, action, entity_type, search) = if form.clear.is_some() {
        (None, None, None, None)
    } else {
        (
            form.actor.filter(|s| !s.is_empty()),
            form.action.filter(|s| !s.is_empty()),
            form.status.filter(|s| !s.is_empty()), // from status tabs
            form.search.filter(|s| !s.is_empty()),
        )
    };

    let filter = AuditQueryFilter {
        actor,
        action,
        entity_type,
        search,
    };

    let cursor_state = CursorState::new(form.cursor.clone(), form.trail.clone());
    let cursor = match cursor_state.decode_with(
        AuditCursor::decode,
        "infra::http::admin::audit::admin_audit_panel",
    ) {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let mut content = match build_audit_list_view(&state, &filter, cursor).await {
        Ok(content) => content,
        Err(err) => {
            return admin_audit_error("infra::http::admin::audit::admin_audit_panel", err)
                .into_response();
        }
    };

    apply_pagination_links(&mut content, &cursor_state);

    match render_audit_panel_html(&content, "infra::http::admin::audit::admin_audit_panel") {
        Ok(html) => datastar_replace(AUDIT_PANEL, html).into_response(),
        Err(err) => err.into_response(),
    }
}

/// GET /audit/{id} - Render audit log detail page.
pub(crate) async fn admin_audit_detail(
    State(state): State<AdminState>,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> Response {
    let record = match state.audit.find_by_id(id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (axum::http::StatusCode::NOT_FOUND, "Audit log not found").into_response();
        }
        Err(err) => {
            return admin_audit_error("infra::http::admin::audit::admin_audit_detail", err)
                .into_response();
        }
    };

    let settings = match state.settings.load().await {
        Ok(s) => s,
        Err(err) => {
            tracing::error!(error = %err, "Failed to load settings for audit detail");
            return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let content = admin_views::AdminAuditDetailView {
        heading: "Audit Log Detail".to_string(),
        fields: vec![
            admin_views::AdminAuditDetailField {
                label: "ID".to_string(),
                value: record.id.to_string(),
                is_badge: false,
                badge_status: None,
                is_multiline: false,
            },
            admin_views::AdminAuditDetailField {
                label: "Actor".to_string(),
                value: record.actor.clone(),
                is_badge: false,
                badge_status: None,
                is_multiline: false,
            },
            admin_views::AdminAuditDetailField {
                label: "Action".to_string(),
                value: record.action.clone(),
                is_badge: true,
                badge_status: Some(record.action.clone()),
                is_multiline: false,
            },
            admin_views::AdminAuditDetailField {
                label: "Entity Type".to_string(),
                value: record.entity_type.clone(),
                is_badge: true,
                badge_status: Some(record.entity_type.clone()),
                is_multiline: false,
            },
            admin_views::AdminAuditDetailField {
                label: "Entity ID".to_string(),
                value: record.entity_id.clone().unwrap_or_else(|| "—".to_string()),
                is_badge: false,
                badge_status: None,
                is_multiline: false,
            },
            admin_views::AdminAuditDetailField {
                label: "Payload".to_string(),
                value: record
                    .payload_text
                    .clone()
                    .unwrap_or_else(|| "—".to_string()),
                is_badge: false,
                badge_status: None,
                is_multiline: true,
            },
            admin_views::AdminAuditDetailField {
                label: "Created At".to_string(),
                value: admin_views::format_timestamp(record.created_at, settings.timezone),
                is_badge: false,
                badge_status: None,
                is_multiline: false,
            },
        ],
        back_href: "/audit".to_string(),
    };

    let chrome = match state.chrome.load("/audit").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };
    let view = admin_views::AdminLayout::new(chrome, content);
    let template = admin_views::AdminAuditDetailTemplate { view };

    match template.render() {
        Ok(html) => axum::response::Html(html).into_response(),
        Err(err) => {
            tracing::error!(error = %err, "Failed to render audit detail template");
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
