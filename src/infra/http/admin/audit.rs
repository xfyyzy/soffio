use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use url::form_urlencoded::Serializer;

use crate::application::pagination::{AuditCursor, PageRequest};
use crate::application::repos::{AuditQueryFilter, RepoError, SettingsRepo};
use crate::{
    application::error::HttpError,
    presentation::{admin::views as admin_views, views::render_template_response},
};

use super::AdminState;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct AdminAuditQuery {
    actor: Option<String>,
    action: Option<String>,
    entity_type: Option<String>,
    search: Option<String>,
    cursor: Option<String>,
}

fn normalize_filter_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn build_audit_filter(query: &AdminAuditQuery) -> AuditQueryFilter {
    AuditQueryFilter {
        actor: normalize_filter_value(query.actor.as_deref()),
        action: normalize_filter_value(query.action.as_deref()),
        entity_type: normalize_filter_value(query.entity_type.as_deref()),
        search: normalize_filter_value(query.search.as_deref()),
    }
}

fn build_audit_filter_query(filter: &AuditQueryFilter) -> String {
    let mut serializer = Serializer::new(String::new());

    if let Some(actor) = filter.actor.as_deref() {
        serializer.append_pair("actor", actor);
    }

    if let Some(action) = filter.action.as_deref() {
        serializer.append_pair("action", action);
    }

    if let Some(entity_type) = filter.entity_type.as_deref() {
        serializer.append_pair("entity_type", entity_type);
    }

    if let Some(search) = filter.search.as_deref() {
        serializer.append_pair("search", search);
    }

    serializer.finish()
}

fn decode_audit_cursor(
    value: Option<&str>,
    source: &'static str,
) -> Result<Option<AuditCursor>, HttpError> {
    match normalize_filter_value(value) {
        Some(raw) => AuditCursor::decode(&raw).map(Some).map_err(|err| {
            HttpError::new(
                source,
                StatusCode::BAD_REQUEST,
                "Invalid cursor",
                err.to_string(),
            )
        }),
        None => Ok(None),
    }
}

pub(super) async fn admin_audit(
    State(state): State<AdminState>,
    Query(query): Query<AdminAuditQuery>,
) -> Response {
    let chrome = match state.chrome.load("/audit").await {
        Ok(chrome) => chrome,
        Err(err) => return err.into_response(),
    };

    let filter = build_audit_filter(&query);

    let cursor = match decode_audit_cursor(query.cursor.as_deref(), "infra::http::admin_audit") {
        Ok(cursor) => cursor,
        Err(err) => return err.into_response(),
    };

    let page_request = PageRequest::new(100, cursor);

    let page = match state.audit.list_filtered(page_request, &filter).await {
        Ok(entries) => entries,
        Err(err) => return admin_audit_error("infra::http::admin_audit", err).into_response(),
    };

    let timezone = match state.db.load_site_settings().await {
        Ok(settings) => settings.timezone,
        Err(err) => {
            return admin_audit_error("infra::http::admin_audit", err).into_response();
        }
    };

    let rows = page
        .items
        .into_iter()
        .map(|entry| admin_views::AdminAuditRowView {
            id: entry.id.to_string(),
            actor: entry.actor,
            action: entry.action,
            entity: match entry.entity_id {
                Some(id) => format!("{}:{id}", entry.entity_type),
                None => entry.entity_type,
            },
            created_at: admin_views::format_timestamp(entry.created_at, timezone),
        })
        .collect();

    let filter_query = build_audit_filter_query(&filter);
    let filter_actor = filter.actor.clone();
    let filter_action = filter.action.clone();
    let filter_entity_type = filter.entity_type.clone();
    let filter_search = filter.search.clone();

    let content = admin_views::AdminAuditListView {
        heading: "Audit Log".to_string(),
        entries: rows,
        filter_actor,
        filter_action,
        filter_entity_type,
        filter_search,
        filter_query,
        next_cursor: page.next_cursor,
    };

    let view = admin_views::AdminLayout::new(chrome, content);
    render_template_response(admin_views::AdminAuditTemplate { view }, StatusCode::OK)
}

fn admin_audit_error(source: &'static str, err: RepoError) -> HttpError {
    HttpError::from_error(
        source,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error",
        &err,
    )
}
