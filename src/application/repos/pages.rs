use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::pagination::{CursorPage, PageCursor};
use crate::domain::entities::PageRecord;
use crate::domain::types::PageStatus;

use super::RepoError;

#[derive(Debug, Clone, Default)]
pub struct PageQueryFilter {
    pub search: Option<String>,
    pub month: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreatePageParams {
    pub slug: String,
    pub title: String,
    pub body_markdown: String,
    pub rendered_html: String,
    pub status: PageStatus,
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
    pub status: PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
pub struct RestorePageSnapshotParams {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub body_markdown: String,
    pub rendered_html: String,
    pub status: PageStatus,
    pub scheduled_at: Option<OffsetDateTime>,
    pub published_at: Option<OffsetDateTime>,
    pub archived_at: Option<OffsetDateTime>,
}

#[async_trait]
pub trait PagesRepo: Send + Sync {
    async fn list_pages(
        &self,
        status: Option<PageStatus>,
        limit: u32,
        cursor: Option<PageCursor>,
        filter: &PageQueryFilter,
    ) -> Result<CursorPage<PageRecord>, RepoError>;

    async fn find_by_slug(&self, slug: &str) -> Result<Option<PageRecord>, RepoError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PageRecord>, RepoError>;

    async fn count_pages(
        &self,
        status: Option<PageStatus>,
        filter: &PageQueryFilter,
    ) -> Result<u64, RepoError>;

    async fn list_month_counts(
        &self,
        status: Option<PageStatus>,
        filter: &PageQueryFilter,
    ) -> Result<Vec<crate::domain::posts::MonthCount>, RepoError>;
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

    async fn restore_page_snapshot(
        &self,
        params: RestorePageSnapshotParams,
    ) -> Result<PageRecord, RepoError>;
}
