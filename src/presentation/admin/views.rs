use crate::util::timezone;
use chrono_tz::Tz;
use time::OffsetDateTime;

mod api_keys;
mod audit;
mod dashboard;
mod editors;
mod jobs;
mod navigation;
mod pages;
mod posts;
mod settings;
mod snapshots;
mod tags;
mod toast;
mod uploads;

pub use api_keys::{
    AdminApiKeyCreatedPanelTemplate, AdminApiKeyCreatedView, AdminApiKeyEditTemplate,
    AdminApiKeyEditorPanelTemplate, AdminApiKeyEditorView, AdminApiKeyExpiresInOption,
    AdminApiKeyListView, AdminApiKeyNewTemplate, AdminApiKeyPaginationState, AdminApiKeyRowView,
    AdminApiKeyScopePickerTemplate, AdminApiKeyScopePickerView,
    AdminApiKeyScopeSelectionStoreTemplate, AdminApiKeyStatusFilterView, AdminApiKeysPanelTemplate,
    AdminApiKeysTemplate, AdminApiScopeDisplay, AdminApiScopeOption,
};
pub use audit::{
    AdminAuditActionOption, AdminAuditActorOption, AdminAuditDetailField, AdminAuditDetailTemplate,
    AdminAuditDetailView, AdminAuditListView, AdminAuditPanelTemplate, AdminAuditRowView,
    AdminAuditStatusFilterView, AdminAuditTemplate,
};
pub use dashboard::{
    AdminDashboardPanelView, AdminDashboardTemplate, AdminDashboardView, AdminMetricView,
};
pub use editors::{
    AdminPageEditPanelTemplate, AdminPageEditTemplate, AdminPageEditorView, AdminPageStatusOption,
    AdminPostEditPanelTemplate, AdminPostEditTemplate, AdminPostEditorView,
    AdminPostSelectedTagView, AdminPostStatusOption, AdminPostTagPickerOptionView,
    AdminPostTagPickerTemplate, AdminPostTagPickerView, AdminPostTagSelectionStoreTemplate,
};
pub use jobs::{
    AdminJobDetailField, AdminJobDetailPanelTemplate, AdminJobDetailTemplate, AdminJobDetailView,
    AdminJobListView, AdminJobRowActionView, AdminJobRowView, AdminJobStatusFilterView,
    AdminJobTypeOption, AdminJobsPanelTemplate, AdminJobsTemplate,
};
pub use navigation::{
    AdminNavigationDestinationTypeOption, AdminNavigationEditPanelTemplate,
    AdminNavigationEditTemplate, AdminNavigationEditorView, AdminNavigationListView,
    AdminNavigationPageOption, AdminNavigationPanelTemplate, AdminNavigationRowView,
    AdminNavigationStatusFilterView, AdminNavigationTemplate,
};
pub use pages::{
    AdminPageListView, AdminPageRowView, AdminPageStatusFilterView, AdminPagesPanelTemplate,
    AdminPagesTemplate,
};
pub use posts::{
    AdminPostListView, AdminPostMonthOption, AdminPostPaginationState, AdminPostRowActionView,
    AdminPostRowView, AdminPostStatusFilterView, AdminPostTagOption, AdminPostTimeKind,
    AdminPostsPanelTemplate, AdminPostsTemplate,
};
pub use settings::{
    AdminSettingsEditInputKind, AdminSettingsEditMultilineField, AdminSettingsEditPanelTemplate,
    AdminSettingsEditSimpleField, AdminSettingsEditTemplate, AdminSettingsEditView,
    AdminSettingsPanelTemplate, AdminSettingsSummaryField, AdminSettingsSummaryValueKind,
    AdminSettingsSummaryView, AdminSettingsTemplate,
};
pub use snapshots::{
    AdminSnapshotEditTemplate, AdminSnapshotEditorPanelTemplate, AdminSnapshotEditorView,
    AdminSnapshotListView, AdminSnapshotNewTemplate, AdminSnapshotRowView,
    AdminSnapshotsPanelTemplate, AdminSnapshotsTemplate,
};
pub use tags::{
    AdminTagEditPanelTemplate, AdminTagEditTemplate, AdminTagEditView, AdminTagListView,
    AdminTagRowView, AdminTagsPanelTemplate, AdminTagsTemplate,
};
pub use toast::{AdminToastItem, AdminToastStackTemplate};
pub use uploads::{
    AdminUploadFormView, AdminUploadListView, AdminUploadNewPanelTemplate, AdminUploadNewTemplate,
    AdminUploadQueueEntry, AdminUploadQueueTemplate, AdminUploadQueueView, AdminUploadRowView,
    AdminUploadsPanelTemplate, AdminUploadsTemplate,
};

#[derive(Clone)]
pub struct AdminBrandView {
    pub title: String,
}

#[derive(Clone)]
pub struct AdminNavigationItemView {
    pub label: String,
    pub href: String,
    pub is_active: bool,
    pub open_in_new_tab: bool,
}

#[derive(Clone)]
pub struct AdminNavigationView {
    pub items: Vec<AdminNavigationItemView>,
}

#[derive(Clone)]
pub struct AdminMetaView {
    pub title: String,
    pub description: String,
}

#[derive(Clone)]
pub struct AdminChrome {
    pub brand: AdminBrandView,
    pub navigation: AdminNavigationView,
    pub meta: AdminMetaView,
}

#[derive(Clone)]
pub struct AdminLayout<T> {
    pub chrome: AdminChrome,
    pub asset_version: String,
    pub content: T,
}

impl<T> AdminLayout<T> {
    pub fn new(chrome: AdminChrome, content: T) -> Self {
        Self {
            chrome,
            asset_version: asset_version(),
            content,
        }
    }
}

fn asset_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Generic hidden field for form submissions.
/// Used by templates to render additional hidden inputs without hardcoding.
#[derive(Clone)]
pub struct AdminHiddenField {
    pub name: String,
    pub value: String,
}

impl AdminHiddenField {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Create from an Option<String>, returning None if value is None or empty.
    pub fn from_option(name: impl Into<String>, value: &Option<String>) -> Option<Self> {
        value.as_ref().filter(|v| !v.is_empty()).map(|v| Self {
            name: name.into(),
            value: v.clone(),
        })
    }
}

pub fn format_timestamp(time: OffsetDateTime, tz: Tz) -> String {
    let localized = timezone::localized_datetime(time, tz);
    localized.format("%Y/%m/%d %H:%M:%S").to_string()
}
