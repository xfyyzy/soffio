use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::pagination::{CursorPage, PageRequest, PostCursor};
use crate::domain::entities::{PostRecord, PostSectionRecord};
use crate::domain::types::PostStatus;

use super::RepoError;

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
pub struct RestorePostSnapshotParams {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub body_markdown: String,
    pub summary_markdown: Option<String>,
    pub summary_html: Option<String>,
    pub status: PostStatus,
    pub pinned: bool,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
    pub tag_ids: Vec<Uuid>,
    pub sections: Vec<crate::application::admin::snapshot_types::PostSectionSnapshot>,
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

#[derive(Debug, Clone)]
pub struct PostTagCount {
    pub slug: String,
    pub name: String,
    pub count: u64,
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

    async fn restore_post_snapshot(
        &self,
        params: RestorePostSnapshotParams,
    ) -> Result<PostRecord, RepoError>;
}

#[async_trait]
pub trait SectionsRepo: Send + Sync {
    async fn list_sections(&self, post_id: Uuid) -> Result<Vec<PostSectionRecord>, RepoError>;
}
