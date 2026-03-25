use super::*;

#[async_trait]
impl NavigationRepo for StaticContentRepo {
    async fn list_navigation(
        &self,
        _visibility: Option<bool>,
        filter: &NavigationQueryFilter,
        page: PageRequest<NavigationCursor>,
    ) -> Result<CursorPage<NavigationItemRecord>, RepoError> {
        let mut records = self.navigation_records();
        if let Some(search) = filter.search.as_ref() {
            let needle = search.to_lowercase();
            records.retain(|r| r.label.to_lowercase().contains(&needle));
        }

        let start = page
            .cursor
            .as_ref()
            .and_then(|cursor| records.iter().position(|nav| nav.id == cursor.id()))
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let limit = page.limit.clamp(1, 100) as usize;
        let slice = records
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_cursor = records
            .get(start + limit)
            .map(|nav| NavigationCursor::new(nav.sort_order, nav.created_at, nav.id).encode());

        Ok(CursorPage::new(slice, next_cursor))
    }

    async fn count_navigation(
        &self,
        _visibility: Option<bool>,
        filter: &NavigationQueryFilter,
    ) -> Result<u64, RepoError> {
        let mut records = self.navigation_records();
        if let Some(search) = filter.search.as_ref() {
            let needle = search.to_lowercase();
            records.retain(|r| r.label.to_lowercase().contains(&needle));
        }
        Ok(records.len() as u64)
    }

    async fn count_external_navigation(
        &self,
        _visibility: Option<bool>,
        filter: &NavigationQueryFilter,
    ) -> Result<u64, RepoError> {
        let mut records = self.navigation_records();
        if let Some(search) = filter.search.as_ref() {
            let needle = search.to_lowercase();
            records.retain(|r| r.label.to_lowercase().contains(&needle));
        }
        Ok(records
            .into_iter()
            .filter(|record| record.destination_type == NavigationDestinationType::External)
            .count() as u64)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<NavigationItemRecord>, RepoError> {
        Ok(self
            .navigation_records()
            .into_iter()
            .find(|record| record.id == id))
    }
}
