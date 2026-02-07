use askama::Template;

use super::{AdminHiddenField, AdminLayout, AdminPostPaginationState};

/// Single audit log entry row view.
#[derive(Clone)]
pub struct AdminAuditRowView {
    pub id: String,
    pub actor: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub payload_text: Option<String>,
    pub created_at: String,
    pub detail_href: String,
}

/// Status filter for audit list (entity type tabs).
#[derive(Clone)]
pub struct AdminAuditStatusFilterView {
    pub status_key: Option<String>,
    pub label: String,
    pub count: usize,
    pub is_active: bool,
}

/// Actor option for dropdown filter.
#[derive(Clone)]
pub struct AdminAuditActorOption {
    pub value: String,
    pub label: String,
    pub count: usize,
}

/// Action option for dropdown filter.
#[derive(Clone)]
pub struct AdminAuditActionOption {
    pub value: String,
    pub label: String,
    pub count: usize,
}

/// Audit list view aligned with AdminPostListView structure.
#[derive(Clone)]
pub struct AdminAuditListView {
    pub heading: String,
    pub filters: Vec<AdminAuditStatusFilterView>,
    pub entries: Vec<AdminAuditRowView>,

    // Audit-specific filter options
    pub actor_options: Vec<AdminAuditActorOption>,
    pub action_options: Vec<AdminAuditActionOption>,

    // Current filter values
    pub filter_actor: Option<String>,
    pub filter_action: Option<String>,
    pub filter_entity_type: Option<String>,
    pub filter_search: Option<String>,
    pub filter_query: String,

    // Status tabs
    pub active_status_key: Option<String>,

    // Pagination
    pub next_cursor: Option<String>,
    pub cursor_param: Option<String>,
    pub trail: Option<String>,
    pub previous_page_state: Option<AdminPostPaginationState>,
    pub next_page_state: Option<AdminPostPaginationState>,

    // Action paths
    pub panel_action: String,

    /// Generic hidden fields for filter state retention
    pub custom_hidden_fields: Vec<AdminHiddenField>,
}

#[derive(Template)]
#[template(path = "admin/audit.html")]
pub struct AdminAuditTemplate {
    pub view: AdminLayout<AdminAuditListView>,
}

#[derive(Template)]
#[template(path = "admin/audit_panel.html")]
pub struct AdminAuditPanelTemplate {
    pub content: AdminAuditListView,
}

/// Audit detail field for display.
#[derive(Clone)]
pub struct AdminAuditDetailField {
    pub label: String,
    pub value: String,
    pub is_badge: bool,
    pub badge_status: Option<String>,
    pub is_multiline: bool,
}

/// Audit detail view for single audit log entry.
#[derive(Clone)]
pub struct AdminAuditDetailView {
    pub heading: String,
    pub fields: Vec<AdminAuditDetailField>,
    pub back_href: String,
}

#[derive(Template)]
#[template(path = "admin/audit_detail.html")]
pub struct AdminAuditDetailTemplate {
    pub view: AdminLayout<AdminAuditDetailView>,
}
