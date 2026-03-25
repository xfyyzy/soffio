use uuid::Uuid;

use crate::application::pagination::{CursorPage, PageRequest, TagCursor};
use crate::application::repos::{TagListRecord, TagQueryFilter, TagWithCount};
use crate::domain::entities::TagRecord;
use crate::domain::posts::MonthCount;

use super::service::AdminTagService;
use super::types::{AdminTagError, AdminTagStatusCounts};

impl AdminTagService {
    pub async fn list_all(&self) -> Result<Vec<TagRecord>, AdminTagError> {
        self.reader.list_all().await.map_err(AdminTagError::from)
    }

    pub async fn list_with_counts(&self) -> Result<Vec<TagWithCount>, AdminTagError> {
        self.reader
            .list_with_counts()
            .await
            .map_err(AdminTagError::from)
    }

    pub async fn list_for_post(&self, post_id: Uuid) -> Result<Vec<TagRecord>, AdminTagError> {
        self.reader
            .list_for_post(post_id)
            .await
            .map_err(AdminTagError::from)
    }

    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<TagRecord>, AdminTagError> {
        self.reader
            .find_by_slug(slug)
            .await
            .map_err(AdminTagError::from)
    }

    pub async fn list(
        &self,
        pinned: Option<bool>,
        filter: &TagQueryFilter,
        page: PageRequest<TagCursor>,
    ) -> Result<CursorPage<TagListRecord>, AdminTagError> {
        self.reader
            .list_admin_tags(pinned, filter, page)
            .await
            .map_err(AdminTagError::from)
    }

    pub async fn status_counts(
        &self,
        filter: &TagQueryFilter,
    ) -> Result<AdminTagStatusCounts, AdminTagError> {
        let total = self.reader.count_tags(None, filter).await?;
        let pinned = self.reader.count_tags(Some(true), filter).await?;
        let unpinned = self.reader.count_tags(Some(false), filter).await?;

        Ok(AdminTagStatusCounts {
            total,
            pinned,
            unpinned,
        })
    }

    pub async fn month_counts(
        &self,
        pinned: Option<bool>,
        filter: &TagQueryFilter,
    ) -> Result<Vec<MonthCount>, AdminTagError> {
        self.reader
            .month_counts(pinned, filter)
            .await
            .map_err(AdminTagError::from)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<TagRecord>, AdminTagError> {
        self.reader
            .find_by_id(id)
            .await
            .map_err(AdminTagError::from)
    }
}
