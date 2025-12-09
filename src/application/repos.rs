//! Repository traits describing persistence adapters.

use async_trait::async_trait;
use serde::Serialize;
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::pagination::{
    ApiKeyCursor, AuditCursor, CursorPage, JobCursor, NavigationCursor, PageCursor, PageRequest,
    PaginationError, PostCursor, TagCursor, UploadCursor,
};
use crate::domain::api_keys::{ApiKeyRecord, ApiScope};
use crate::domain::entities::{
    AuditLogRecord, JobRecord, NavigationItemRecord, PageRecord, PostRecord, PostSectionRecord,
    SiteSettingsRecord, TagRecord, UploadRecord,
};
use crate::domain::types::{JobState, JobType, NavigationDestinationType, PostStatus};

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("persistence error: {0}")]
    Persistence(String),
    #[error("duplicate record violates unique constraint `{constraint}`")]
    Duplicate { constraint: String },
    #[error("resource not found")]
    NotFound,
    #[error("invalid input: {message}")]
    InvalidInput { message: String },
    #[error("integrity error: {message}")]
    Integrity { message: String },
    #[error("database timeout")]
    Timeout,
    #[error(transparent)]
    Pagination(#[from] PaginationError),
}

impl RepoError {
    pub fn from_persistence(err: impl std::fmt::Display) -> Self {
        Self::Persistence(err.to_string())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PostListScope {
    Public,
    Admin { status: Option<PostStatus> },
}

#[derive(Debug, Clone, Default)]
pub struct PostQueryFilter {
    pub tag: Option<String>,
    pub month: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PageQueryFilter {
    pub search: Option<String>,
    pub month: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TagQueryFilter {
    pub search: Option<String>,
    pub month: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UploadQueryFilter {
    pub content_type: Option<String>,
    pub month: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NavigationQueryFilter {
    pub search: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct JobQueryFilter {
    pub state: Option<JobState>,
    pub job_type: Option<JobType>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AuditQueryFilter {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreatePostParams {
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub body_markdown: String,
    pub status: PostStatus,
    pub pinned: bool,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
    pub summary_markdown: Option<String>,
    pub summary_html: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdatePostParams {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub body_markdown: String,
    pub pinned: bool,
    pub summary_markdown: Option<String>,
    pub summary_html: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdatePostStatusParams {
    pub id: Uuid,
    pub status: PostStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy)]
pub struct UpdatePostPinnedParams {
    pub id: Uuid,
    pub pinned: bool,
}

#[async_trait]
pub trait PostsRepo: Send + Sync {
    async fn list_posts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
        page: PageRequest<PostCursor>,
    ) -> Result<CursorPage<PostRecord>, RepoError>;

    async fn count_posts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<u64, RepoError>;

    async fn count_posts_before(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
        cursor: &PostCursor,
    ) -> Result<u64, RepoError>;

    async fn list_month_counts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<Vec<crate::domain::posts::MonthCount>, RepoError>;

    async fn list_tag_counts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<Vec<PostTagCount>, RepoError>;

    async fn find_by_slug(&self, slug: &str) -> Result<Option<PostRecord>, RepoError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PostRecord>, RepoError>;
}

#[async_trait]
pub trait PostsWriteRepo: Send + Sync {
    async fn create_post(&self, params: CreatePostParams) -> Result<PostRecord, RepoError>;

    async fn update_post(&self, params: UpdatePostParams) -> Result<PostRecord, RepoError>;

    async fn update_post_status(
        &self,
        params: UpdatePostStatusParams,
    ) -> Result<PostRecord, RepoError>;

    async fn update_post_pinned(
        &self,
        params: UpdatePostPinnedParams,
    ) -> Result<PostRecord, RepoError>;

    async fn schedule_post_publication(
        &self,
        id: Uuid,
        publish_at: OffsetDateTime,
    ) -> Result<PostRecord, RepoError>;

    async fn delete_post(&self, id: Uuid) -> Result<(), RepoError>;

    async fn replace_post_tags(&self, post_id: Uuid, tag_ids: &[Uuid]) -> Result<(), RepoError>;
}

#[async_trait]
pub trait SectionsRepo: Send + Sync {
    async fn list_sections(&self, post_id: Uuid) -> Result<Vec<PostSectionRecord>, RepoError>;
}

#[async_trait]
pub trait PagesRepo: Send + Sync {
    async fn list_pages(
        &self,
        status: Option<crate::domain::types::PageStatus>,
        limit: u32,
        cursor: Option<PageCursor>,
        filter: &PageQueryFilter,
    ) -> Result<CursorPage<PageRecord>, RepoError>;

    async fn find_by_slug(&self, slug: &str) -> Result<Option<PageRecord>, RepoError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PageRecord>, RepoError>;

    async fn count_pages(
        &self,
        status: Option<crate::domain::types::PageStatus>,
        filter: &PageQueryFilter,
    ) -> Result<u64, RepoError>;

    async fn list_month_counts(
        &self,
        status: Option<crate::domain::types::PageStatus>,
        filter: &PageQueryFilter,
    ) -> Result<Vec<crate::domain::posts::MonthCount>, RepoError>;
}

#[derive(Debug, Clone)]
pub struct CreatePageParams {
    pub slug: String,
    pub title: String,
    pub body_markdown: String,
    pub rendered_html: String,
    pub status: crate::domain::types::PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
pub struct UpdatePageParams {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub body_markdown: String,
    pub rendered_html: String,
}

#[derive(Debug, Clone)]
pub struct UpdatePageStatusParams {
    pub id: Uuid,
    pub status: crate::domain::types::PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[async_trait]
pub trait PagesWriteRepo: Send + Sync {
    async fn create_page(&self, params: CreatePageParams) -> Result<PageRecord, RepoError>;

    async fn update_page(&self, params: UpdatePageParams) -> Result<PageRecord, RepoError>;

    async fn update_page_status(
        &self,
        params: UpdatePageStatusParams,
    ) -> Result<PageRecord, RepoError>;

    async fn schedule_page_publication(
        &self,
        id: Uuid,
        publish_at: OffsetDateTime,
    ) -> Result<PageRecord, RepoError>;

    async fn delete_page(&self, id: Uuid) -> Result<(), RepoError>;
}

#[async_trait]
pub trait TagsRepo: Send + Sync {
    async fn list_all(&self) -> Result<Vec<TagRecord>, RepoError>;
    async fn list_for_post(&self, post_id: Uuid) -> Result<Vec<TagRecord>, RepoError>;
    async fn list_with_counts(&self) -> Result<Vec<TagWithCount>, RepoError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<TagRecord>, RepoError>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<TagRecord>, RepoError>;
    async fn count_usage(&self, id: Uuid) -> Result<u64, RepoError>;
    async fn list_admin_tags(
        &self,
        pinned: Option<bool>,
        filter: &TagQueryFilter,
        page: PageRequest<TagCursor>,
    ) -> Result<CursorPage<TagListRecord>, RepoError>;
    async fn count_tags(
        &self,
        pinned: Option<bool>,
        filter: &TagQueryFilter,
    ) -> Result<u64, RepoError>;
    async fn month_counts(
        &self,
        pinned: Option<bool>,
        filter: &TagQueryFilter,
    ) -> Result<Vec<crate::domain::posts::MonthCount>, RepoError>;
}

#[derive(Debug, Clone)]
pub struct CreateTagParams {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateTagParams {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub pinned: bool,
}

#[async_trait]
pub trait TagsWriteRepo: Send + Sync {
    async fn create_tag(&self, params: CreateTagParams) -> Result<TagRecord, RepoError>;

    async fn update_tag(&self, params: UpdateTagParams) -> Result<TagRecord, RepoError>;

    async fn delete_tag(&self, id: Uuid) -> Result<(), RepoError>;
}

#[async_trait]
pub trait NavigationRepo: Send + Sync {
    async fn list_navigation(
        &self,
        visibility: Option<bool>,
        filter: &NavigationQueryFilter,
        page: PageRequest<NavigationCursor>,
    ) -> Result<CursorPage<NavigationItemRecord>, RepoError>;
    async fn count_navigation(
        &self,
        visibility: Option<bool>,
        filter: &NavigationQueryFilter,
    ) -> Result<u64, RepoError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<NavigationItemRecord>, RepoError>;
}

#[derive(Debug, Clone)]
pub struct CreateNavigationItemParams {
    pub label: String,
    pub destination_type: NavigationDestinationType,
    pub destination_page_id: Option<Uuid>,
    pub destination_url: Option<String>,
    pub sort_order: i32,
    pub visible: bool,
    pub open_in_new_tab: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateNavigationItemParams {
    pub id: Uuid,
    pub label: String,
    pub destination_type: NavigationDestinationType,
    pub destination_page_id: Option<Uuid>,
    pub destination_url: Option<String>,
    pub sort_order: i32,
    pub visible: bool,
    pub open_in_new_tab: bool,
}

#[async_trait]
pub trait NavigationWriteRepo: Send + Sync {
    async fn create_navigation_item(
        &self,
        params: CreateNavigationItemParams,
    ) -> Result<NavigationItemRecord, RepoError>;

    async fn update_navigation_item(
        &self,
        params: UpdateNavigationItemParams,
    ) -> Result<NavigationItemRecord, RepoError>;

    async fn delete_navigation_item(&self, id: Uuid) -> Result<(), RepoError>;
}

#[async_trait]
pub trait SettingsRepo: Send + Sync {
    async fn load_site_settings(&self) -> Result<SiteSettingsRecord, RepoError>;
    async fn upsert_site_settings(&self, settings: SiteSettingsRecord) -> Result<(), RepoError>;
}

#[async_trait]
pub trait UploadsRepo: Send + Sync {
    async fn insert_upload(&self, record: UploadRecord) -> Result<(), RepoError>;
    async fn find_upload(&self, id: Uuid) -> Result<Option<UploadRecord>, RepoError>;
    async fn list_recent(
        &self,
        limit: u32,
        before: Option<OffsetDateTime>,
    ) -> Result<Vec<UploadRecord>, RepoError>;
    async fn list_uploads(
        &self,
        filter: &UploadQueryFilter,
        page: PageRequest<UploadCursor>,
    ) -> Result<CursorPage<UploadRecord>, RepoError>;
    async fn count_uploads(&self, filter: &UploadQueryFilter) -> Result<u64, RepoError>;
    async fn month_counts(
        &self,
        filter: &UploadQueryFilter,
    ) -> Result<Vec<UploadMonthCount>, RepoError>;
    async fn content_type_counts(
        &self,
        filter: &UploadQueryFilter,
    ) -> Result<Vec<UploadContentTypeCount>, RepoError>;
    async fn delete_upload(&self, id: Uuid) -> Result<(), RepoError>;
}

#[async_trait]
pub trait AuditRepo: Send + Sync {
    async fn append_log(&self, record: AuditLogRecord) -> Result<(), RepoError>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<AuditLogRecord>, RepoError>;
    async fn list_filtered(
        &self,
        page: PageRequest<AuditCursor>,
        filter: &AuditQueryFilter,
    ) -> Result<CursorPage<AuditLogRecord>, RepoError>;
}

#[derive(Debug, Clone)]
pub struct NewJobRecord {
    pub job_type: JobType,
    pub payload: serde_json::Value,
    pub run_at: OffsetDateTime,
    pub max_attempts: i32,
    pub priority: i32,
}

#[derive(Debug, Clone)]
pub struct UpdateJobStateParams {
    pub id: String,
    pub state: JobState,
    pub last_error: Option<String>,
    pub attempts: Option<i32>,
    pub run_at: Option<OffsetDateTime>,
    pub priority: Option<i32>,
}

#[async_trait]
pub trait JobsRepo: Send + Sync {
    async fn enqueue_job(&self, job: NewJobRecord) -> Result<String, RepoError>;

    async fn update_job_state(&self, params: UpdateJobStateParams) -> Result<(), RepoError>;

    async fn find_job(&self, id: &str) -> Result<Option<JobRecord>, RepoError>;

    async fn list_jobs(
        &self,
        filter: &JobQueryFilter,
        page: PageRequest<JobCursor>,
    ) -> Result<CursorPage<JobRecord>, RepoError>;

    async fn count_jobs(&self, filter: &JobQueryFilter) -> Result<u64, RepoError>;
}

#[derive(Debug, Clone, Serialize)]
pub struct TagWithCount {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub pinned: bool,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagListRecord {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub pinned: bool,
    pub usage_count: u64,
    pub primary_time: OffsetDateTime,
    pub updated_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct PostTagCount {
    pub slug: String,
    pub name: String,
    pub count: u64,
}

#[derive(Debug, Clone)]
pub struct UploadContentTypeCount {
    pub content_type: String,
    pub count: u64,
}

#[derive(Debug, Clone)]
pub struct UploadMonthCount {
    pub key: String,
    pub label: String,
    pub count: u64,
}

#[derive(Debug, Clone)]
pub struct CreateApiKeyParams {
    pub name: String,
    pub description: Option<String>,
    pub prefix: String,
    pub hashed_secret: Vec<u8>,
    pub scopes: Vec<ApiScope>,
    pub expires_in: Option<time::Duration>,
    pub expires_at: Option<OffsetDateTime>,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct UpdateApiKeySecretParams {
    pub id: Uuid,
    pub new_prefix: String,
    pub new_hashed_secret: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct UpdateApiKeyMetadataParams {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<ApiScope>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyStatusFilter {
    Active,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, Default)]
pub struct ApiKeyQueryFilter {
    pub status: Option<ApiKeyStatusFilter>,
    pub scope: Option<ApiScope>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ApiKeyPageRequest {
    pub limit: u32,
    pub cursor: Option<ApiKeyCursor>,
}

#[derive(Debug, Clone)]
pub struct ApiKeyListPage {
    pub items: Vec<ApiKeyRecord>,
    pub total: u64,
    pub active: u64,
    pub revoked: u64,
    pub expired: u64,
    pub next_cursor: Option<ApiKeyCursor>,
    pub scope_counts: Vec<(ApiScope, u64)>,
}

#[async_trait]
pub trait ApiKeysRepo: Send + Sync {
    async fn create_key(&self, params: CreateApiKeyParams) -> Result<ApiKeyRecord, RepoError>;

    async fn list_keys(
        &self,
        filter: &ApiKeyQueryFilter,
        page: ApiKeyPageRequest,
    ) -> Result<ApiKeyListPage, RepoError>;

    async fn find_by_prefix(&self, prefix: &str) -> Result<Option<ApiKeyRecord>, RepoError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ApiKeyRecord>, RepoError>;

    async fn revoke_key(&self, id: Uuid, revoked_at: OffsetDateTime) -> Result<(), RepoError>;

    async fn delete_key(&self, id: Uuid) -> Result<bool, RepoError>;

    async fn expire_keys(&self) -> Result<u64, RepoError>;

    async fn update_secret(
        &self,
        params: UpdateApiKeySecretParams,
    ) -> Result<ApiKeyRecord, RepoError>;

    async fn update_metadata(
        &self,
        params: UpdateApiKeyMetadataParams,
    ) -> Result<ApiKeyRecord, RepoError>;

    async fn update_last_used(
        &self,
        id: Uuid,
        last_used_at: OffsetDateTime,
    ) -> Result<(), RepoError>;
}
