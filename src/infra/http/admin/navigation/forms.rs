//! Navigation admin form definitions.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct AdminNavigationQuery {
    pub(super) status: Option<String>,
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
    pub(super) search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminNavigationPanelForm {
    pub(super) status: Option<String>,
    pub(super) search: Option<String>,
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
    pub(super) clear: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminNavigationForm {
    pub(super) label: String,
    pub(super) destination_type: String,
    pub(super) destination_page_id: Option<String>,
    pub(super) destination_url: Option<String>,
    pub(super) sort_order: i32,
    pub(super) visible: Option<String>,
    pub(super) open_in_new_tab: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminNavigationDeleteForm {
    pub(super) status: Option<String>,
    pub(super) search: Option<String>,
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminNavigationVisibilityForm {
    pub(super) status: Option<String>,
    pub(super) search: Option<String>,
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
}
