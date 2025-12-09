//! Form structs for jobs admin handlers.

use serde::Deserialize;

/// Form for status/type action requests with filter context.
#[derive(Debug, Deserialize)]
pub(crate) struct AdminJobActionForm {
    pub(crate) status_filter: Option<String>,
    pub(crate) filter_job_type: Option<String>,
    pub(crate) filter_search: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) trail: Option<String>,
}

/// Form for panel refresh requests.
#[derive(Debug, Deserialize)]
pub(crate) struct AdminJobsPanelForm {
    pub(crate) status: Option<String>,
    pub(crate) job_type: Option<String>,
    pub(crate) search: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) trail: Option<String>,
    pub(crate) clear: Option<String>,
}
