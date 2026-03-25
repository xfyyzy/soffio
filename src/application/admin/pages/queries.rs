use uuid::Uuid;

use crate::{
    application::{
        pagination::{CursorPage, PageCursor},
        repos::PageQueryFilter,
    },
    domain::{entities::PageRecord, types::PageStatus},
};

use super::{
    service::AdminPageService,
    types::{AdminPageError, AdminPageStatusCounts},
};

impl AdminPageService {
    pub async fn list(
        &self,
        status: Option<PageStatus>,
        limit: u32,
        cursor: Option<PageCursor>,
        filter: &PageQueryFilter,
    ) -> Result<CursorPage<PageRecord>, AdminPageError> {
        self.reader
            .list_pages(status, limit, cursor, filter)
            .await
            .map_err(AdminPageError::from)
    }

    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<PageRecord>, AdminPageError> {
        self.reader
            .find_by_slug(slug)
            .await
            .map_err(AdminPageError::from)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<PageRecord>, AdminPageError> {
        self.reader
            .find_by_id(id)
            .await
            .map_err(AdminPageError::from)
    }

    pub async fn status_counts(
        &self,
        filter: &PageQueryFilter,
    ) -> Result<AdminPageStatusCounts, AdminPageError> {
        let total_filter = filter.clone();
        let draft_filter = filter.clone();
        let published_filter = filter.clone();
        let archived_filter = filter.clone();
        let error_filter = filter.clone();

        let total_fut = self.reader.count_pages(None, &total_filter);
        let draft_fut = self
            .reader
            .count_pages(Some(PageStatus::Draft), &draft_filter);
        let published_fut = self
            .reader
            .count_pages(Some(PageStatus::Published), &published_filter);
        let archived_fut = self
            .reader
            .count_pages(Some(PageStatus::Archived), &archived_filter);
        let error_fut = self
            .reader
            .count_pages(Some(PageStatus::Error), &error_filter);

        let (total, draft, published, archived, error) =
            tokio::try_join!(total_fut, draft_fut, published_fut, archived_fut, error_fut)?;

        Ok(AdminPageStatusCounts {
            total,
            draft,
            published,
            archived,
            error,
        })
    }

    pub async fn month_counts(
        &self,
        status: Option<PageStatus>,
        filter: &PageQueryFilter,
    ) -> Result<Vec<crate::domain::posts::MonthCount>, AdminPageError> {
        self.reader
            .list_month_counts(status, filter)
            .await
            .map_err(AdminPageError::from)
    }
}
