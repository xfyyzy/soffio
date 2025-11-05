use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct AdminPageForm {
    pub(crate) title: String,
    pub(crate) body_markdown: String,
    pub(crate) status: String,
    pub(crate) status_filter: Option<String>,
    pub(crate) filter_search: Option<String>,
    pub(crate) filter_month: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminPageDeleteForm {
    pub(crate) status_filter: Option<String>,
    pub(crate) filter_search: Option<String>,
    pub(crate) filter_month: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) trail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminPagePanelForm {
    pub(crate) status: Option<String>,
    pub(crate) search: Option<String>,
    pub(crate) month: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) trail: Option<String>,
    pub(crate) clear: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminPageStatusActionForm {
    pub(crate) status_filter: Option<String>,
    pub(crate) filter_search: Option<String>,
    pub(crate) filter_month: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) trail: Option<String>,
}
