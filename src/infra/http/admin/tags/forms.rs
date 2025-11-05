use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct AdminTagForm {
    pub(super) name: String,
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) pinned: Option<String>,
    pub(super) status_filter: Option<String>,
    pub(super) filter_search: Option<String>,
    pub(super) filter_month: Option<String>,
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminTagPanelForm {
    pub(super) status: Option<String>,
    pub(super) search: Option<String>,
    pub(super) month: Option<String>,
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
    pub(super) clear: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminTagDeleteForm {
    pub(super) name: String,
    pub(super) status_filter: Option<String>,
    pub(super) filter_search: Option<String>,
    pub(super) filter_month: Option<String>,
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminTagPinForm {
    pub(super) status_filter: Option<String>,
    pub(super) filter_search: Option<String>,
    pub(super) filter_month: Option<String>,
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
}
