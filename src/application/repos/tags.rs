use async_trait::async_trait;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::pagination::{CursorPage, PageRequest, TagCursor};
use crate::domain::entities::TagRecord;

use super::RepoError;

#[derive(Debug, Clone, Default)]
pub struct TagQueryFilter {
    pub search: Option<String>,
    pub month: Option<String>,
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

#[async_trait]
pub trait TagsWriteRepo: Send + Sync {
    async fn create_tag(&self, params: CreateTagParams) -> Result<TagRecord, RepoError>;

    async fn update_tag(&self, params: UpdateTagParams) -> Result<TagRecord, RepoError>;

    async fn delete_tag(&self, id: Uuid) -> Result<(), RepoError>;
}
