//! Form structs for audit admin handlers.

use serde::Deserialize;

/// Form for panel refresh requests.
#[derive(Debug, Deserialize)]
pub(crate) struct AdminAuditPanelForm {
    /// Entity type from status tabs (mapped to entity_type filter)
    pub(crate) status: Option<String>,
    pub(crate) actor: Option<String>,
    pub(crate) action: Option<String>,
    pub(crate) search: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) trail: Option<String>,
    pub(crate) clear: Option<String>,
}
