//! Repository traits describing persistence adapters.

mod api_keys;
mod audit;
mod error;
mod jobs;
mod navigation;
mod pages;
mod posts;
mod settings;
mod snapshots;
mod tags;
mod uploads;

pub use api_keys::{
    ApiKeyListPage, ApiKeyPageRequest, ApiKeyQueryFilter, ApiKeyStatusFilter, ApiKeysRepo,
    CreateApiKeyParams, UpdateApiKeyMetadataParams, UpdateApiKeySecretParams,
};
pub use audit::{
    AuditActionCount, AuditActorCount, AuditEntityTypeCount, AuditQueryFilter, AuditRepo,
};
pub use error::RepoError;
pub use jobs::{JobQueryFilter, JobsRepo, NewJobRecord, UpdateJobStateParams};
pub use navigation::{
    CreateNavigationItemParams, NavigationQueryFilter, NavigationRepo, NavigationWriteRepo,
    UpdateNavigationItemParams,
};
pub use pages::{
    CreatePageParams, PageQueryFilter, PagesRepo, PagesWriteRepo, RestorePageSnapshotParams,
    UpdatePageParams, UpdatePageStatusParams,
};
pub use posts::{
    CreatePostParams, PostListScope, PostQueryFilter, PostTagCount, PostsRepo, PostsWriteRepo,
    RestorePostSnapshotParams, SectionsRepo, UpdatePostParams, UpdatePostPinnedParams,
    UpdatePostStatusParams,
};
pub use settings::SettingsRepo;
pub use snapshots::{
    SnapshotCursor, SnapshotFilter, SnapshotMonthCount, SnapshotRecord, SnapshotsRepo,
};
pub use tags::{
    CreateTagParams, TagListRecord, TagQueryFilter, TagWithCount, TagsRepo, TagsWriteRepo,
    UpdateTagParams,
};
pub use uploads::{UploadContentTypeCount, UploadMonthCount, UploadQueryFilter, UploadsRepo};
