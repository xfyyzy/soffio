use super::*;

#[async_trait]
impl PagesRepo for StaticContentRepo {
    async fn list_pages(
        &self,
        _status: Option<PageStatus>,
        limit: u32,
        cursor: Option<PageCursor>,
        _filter: &PageQueryFilter,
    ) -> Result<CursorPage<PageRecord>, RepoError> {
        let mut pages = self.page_records();
        pages.sort_by_key(|p| p.created_at);

        let start = cursor
            .as_ref()
            .and_then(|c| pages.iter().position(|p| p.id == c.id()))
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let limit = limit.clamp(1, 100) as usize;
        let slice = pages
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_cursor = pages
            .get(start + limit)
            .map(|p| PageCursor::new(p.created_at, p.id).encode());

        Ok(CursorPage::new(slice, next_cursor))
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<PageRecord>, RepoError> {
        Ok(self
            .page_records()
            .into_iter()
            .find(|record| record.slug == slug))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PageRecord>, RepoError> {
        Ok(self
            .page_records()
            .into_iter()
            .find(|record| record.id == id))
    }

    async fn count_pages(
        &self,
        _status: Option<PageStatus>,
        _filter: &PageQueryFilter,
    ) -> Result<u64, RepoError> {
        Ok(0)
    }

    async fn list_month_counts(
        &self,
        _status: Option<PageStatus>,
        _filter: &PageQueryFilter,
    ) -> Result<Vec<posts::MonthCount>, RepoError> {
        Ok(Vec::new())
    }
}
