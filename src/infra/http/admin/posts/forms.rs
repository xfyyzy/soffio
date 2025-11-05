use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(crate) struct AdminPostStatusActionForm {
    pub(crate) status_filter: Option<String>,
    pub(crate) filter_search: Option<String>,
    pub(crate) filter_tag: Option<String>,
    pub(crate) filter_month: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) trail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminPostForm {
    pub(crate) title: String,
    pub(crate) excerpt: String,
    pub(crate) body_markdown: String,
    pub(crate) summary_markdown: Option<String>,
    pub(crate) status: String,
    pub(crate) status_filter: Option<String>,
    pub(crate) filter_search: Option<String>,
    pub(crate) filter_tag: Option<String>,
    pub(crate) filter_month: Option<String>,
    pub(crate) tag_state: Option<String>,
    #[serde(default)]
    pub(crate) pinned: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminPostDeleteForm {
    pub(crate) status_filter: Option<String>,
    pub(crate) filter_search: Option<String>,
    pub(crate) filter_tag: Option<String>,
    pub(crate) filter_month: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) trail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminPostPinForm {
    pub(crate) status_filter: Option<String>,
    pub(crate) filter_search: Option<String>,
    pub(crate) filter_tag: Option<String>,
    pub(crate) filter_month: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) trail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminPostBulkActionForm {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) ids: Vec<Uuid>,
    pub(crate) status_filter: Option<String>,
    pub(crate) filter_search: Option<String>,
    pub(crate) filter_tag: Option<String>,
    pub(crate) filter_month: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminPostPanelForm {
    pub(crate) status: Option<String>,
    pub(crate) search: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) month: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) trail: Option<String>,
    pub(crate) clear: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminPostTagsToggleForm {
    pub(crate) tag_id: Uuid,
    pub(crate) tag_state: Option<String>,
}
