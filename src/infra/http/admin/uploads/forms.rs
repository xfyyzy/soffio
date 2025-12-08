//! Form definitions for upload admin handlers.

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct AdminUploadQuery {
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
    pub(super) search: Option<String>,
    #[serde(rename = "content_type")]
    pub(super) content_type: Option<String>,
    pub(super) month: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminUploadPanelForm {
    pub(super) search: Option<String>,
    #[serde(rename = "content_type")]
    pub(super) content_type: Option<String>,
    pub(super) month: Option<String>,
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
    pub(super) clear: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminUploadDeleteForm {
    pub(super) cursor: Option<String>,
    pub(super) trail: Option<String>,
    pub(super) search: Option<String>,
    #[serde(rename = "content_type")]
    pub(super) content_type: Option<String>,
    pub(super) month: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UploadQueuePreviewForm {
    #[serde(default)]
    pub(super) queue_manifest: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct UploadQueueManifestEntry {
    pub(super) id: Option<String>,
    pub(super) filename: Option<String>,
    pub(super) size_bytes: Option<u64>,
    pub(super) status: Option<String>,
    pub(super) message: Option<String>,
}
