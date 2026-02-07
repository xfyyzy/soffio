use askama::Template;

use super::{AdminHiddenField, AdminLayout, AdminPostPaginationState};

/// Status filter tab view for jobs.
#[derive(Clone)]
pub struct AdminJobStatusFilterView {
    pub status_key: Option<String>,
    pub label: String,
    pub count: u64,
    pub is_active: bool,
}

/// Job type filter dropdown option.
#[derive(Clone)]
pub struct AdminJobTypeOption {
    pub value: String,
    pub label: String,
    pub count: u64,
}

/// Row action button for job operations.
#[derive(Clone)]
pub struct AdminJobRowActionView {
    pub value: &'static str,
    pub label: &'static str,
    pub is_danger: bool,
}

/// Single job row in the list.
#[derive(Clone)]
pub struct AdminJobRowView {
    pub id: String,
    pub detail_href: String,
    pub job_type_key: String,
    pub job_type_label: String,
    pub state_key: String,
    pub state_label: String,
    pub attempts: String,
    pub run_at: String,
    pub done_at: Option<String>,
    pub last_error: Option<String>,
    pub actions: Vec<AdminJobRowActionView>,
}

/// Complete list view aligned with AdminPostListView pattern.
#[derive(Clone)]
pub struct AdminJobListView {
    pub heading: String,
    pub filters: Vec<AdminJobStatusFilterView>,
    pub jobs: Vec<AdminJobRowView>,

    // Filter state
    pub filter_job_type: Option<String>,
    pub filter_search: Option<String>,
    pub filter_query: String,
    pub active_status_key: Option<String>,

    // Job type filter
    pub job_type_options: Vec<AdminJobTypeOption>,
    pub job_type_filter_enabled: bool,

    // Pagination - reuse AdminPostPaginationState
    pub next_cursor: Option<String>,
    pub cursor_param: Option<String>,
    pub trail: Option<String>,
    pub previous_page_state: Option<AdminPostPaginationState>,
    pub next_page_state: Option<AdminPostPaginationState>,

    // Action paths
    pub panel_action: String,
    pub row_action_prefix: String,

    /// Generic hidden fields for filter state retention
    pub custom_hidden_fields: Vec<AdminHiddenField>,
}

#[derive(Template)]
#[template(path = "admin/jobs.html")]
pub struct AdminJobsTemplate {
    pub view: AdminLayout<AdminJobListView>,
}

#[derive(Template)]
#[template(path = "admin/jobs_panel.html")]
pub struct AdminJobsPanelTemplate {
    pub content: AdminJobListView,
}

/// Job detail field for display.
#[derive(Clone)]
pub struct AdminJobDetailField {
    pub label: String,
    pub value: String,
    pub is_badge: bool,
    pub badge_status: Option<String>,
    pub is_multiline: bool,
}

/// Job detail view for single job page.
#[derive(Clone)]
pub struct AdminJobDetailView {
    pub heading: String,
    pub fields: Vec<AdminJobDetailField>,
}

#[derive(Template)]
#[template(path = "admin/job_detail.html")]
pub struct AdminJobDetailTemplate {
    pub view: AdminLayout<AdminJobDetailView>,
}

#[derive(Template)]
#[template(path = "admin/job_detail_panel.html")]
pub struct AdminJobDetailPanelTemplate {
    pub content: AdminJobDetailView,
}
