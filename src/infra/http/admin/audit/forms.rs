//! Form structs for audit admin handlers.

use serde::Deserialize;

/// Form for panel refresh requests.
#[derive(Debug, Deserialize)]
pub(crate) struct AdminAuditPanelForm {
    pub(crate) actor: Option<String>,
    pub(crate) action: Option<String>,
    pub(crate) entity_type: Option<String>,
    pub(crate) search: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) trail: Option<String>,
    pub(crate) clear: Option<String>,
}
