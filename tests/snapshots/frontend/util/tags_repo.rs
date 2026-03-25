use super::*;

#[async_trait]
impl TagsRepo for StaticContentRepo {
    async fn list_all(&self) -> Result<Vec<TagRecord>, RepoError> {
        Ok(self.tag_records())
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<TagRecord>, RepoError> {
        Ok(self.tag_records().into_iter().find(|tag| tag.slug == slug))
    }

    async fn count_usage(&self, id: Uuid) -> Result<u64, RepoError> {
        let slug = self
            .tag_records()
            .into_iter()
            .find(|tag| tag.id == id)
            .map(|t| t.slug)
            .unwrap_or_default();
        Ok(self
            .all_posts()
            .into_iter()
            .filter(|post| post.tags.contains(&slug.as_str()))
            .count() as u64)
    }

    async fn list_admin_tags(
        &self,
        _pinned: Option<bool>,
        filter: &TagQueryFilter,
        page: PageRequest<TagCursor>,
    ) -> Result<CursorPage<TagListRecord>, RepoError> {
        let mut tags = self.tag_records();

        if let Some(search) = &filter.search {
            let needle = search.to_lowercase();
            tags.retain(|tag| {
                tag.slug.contains(&needle) || tag.name.to_lowercase().contains(&needle)
            });
        }

        let start = page
            .cursor
            .as_ref()
            .and_then(|cursor| tags.iter().position(|tag| tag.id == cursor.id()))
            .map(|idx| idx + 1)
            .unwrap_or(0);

        let limit = page.limit.clamp(1, 100) as usize;
        let slice = tags.iter().skip(start).take(limit);
        let records: Vec<TagListRecord> = slice
            .map(|tag| TagListRecord {
                id: tag.id,
                slug: tag.slug.clone(),
                name: tag.name.clone(),
                description: tag.description.clone(),
                pinned: tag.pinned,
                usage_count: self
                    .all_posts()
                    .into_iter()
                    .filter(|post| post.tags.contains(&tag.slug.as_str()))
                    .count() as u64,
                primary_time: tag.created_at,
                updated_at: Some(tag.updated_at),
                created_at: tag.created_at,
            })
            .collect();

        let next_cursor = tags
            .get(start + limit)
            .map(|tag| TagCursor::new(tag.pinned, tag.created_at, tag.id).encode());

        Ok(CursorPage::new(records, next_cursor))
    }

    async fn count_tags(
        &self,
        _pinned: Option<bool>,
        filter: &TagQueryFilter,
    ) -> Result<u64, RepoError> {
        let mut tags = self.tag_records();
        if let Some(search) = &filter.search {
            let needle = search.to_lowercase();
            tags.retain(|tag| {
                tag.slug.contains(&needle) || tag.name.to_lowercase().contains(&needle)
            });
        }
        Ok(tags.len() as u64)
    }

    async fn month_counts(
        &self,
        _pinned: Option<bool>,
        filter: &TagQueryFilter,
    ) -> Result<Vec<posts::MonthCount>, RepoError> {
        let mut months: BTreeMap<String, (String, usize)> = BTreeMap::new();
        for post in self.filtered_posts(&PostQueryFilter {
            tag: filter.search.clone(),
            ..PostQueryFilter::default()
        }) {
            let key = posts::month_key_for(post.date);
            let label = posts::month_label_for(post.date);
            months
                .entry(key)
                .and_modify(|entry| entry.1 += 1)
                .or_insert((label, 1));
        }

        let mut items = months
            .into_iter()
            .map(|(key, (label, count))| posts::MonthCount { key, label, count })
            .collect::<Vec<_>>();
        items.sort_by(|a, b| b.key.cmp(&a.key));
        Ok(items)
    }
    async fn find_by_id(&self, id: Uuid) -> Result<Option<TagRecord>, RepoError> {
        Ok(self.tag_records().into_iter().find(|tag| tag.id == id))
    }

    async fn list_for_post(&self, post_id: Uuid) -> Result<Vec<TagRecord>, RepoError> {
        let post = self
            .all_posts()
            .into_iter()
            .find(|post| Self::post_uuid(post.slug) == post_id)
            .expect("post exists");
        Ok(post.tags.iter().map(|slug| self.tag_record(slug)).collect())
    }

    async fn list_with_counts(&self) -> Result<Vec<TagWithCount>, RepoError> {
        let mut counts: BTreeMap<&str, i64> = BTreeMap::new();
        for post in posts::all() {
            for tag in post.tags {
                *counts.entry(tag).or_default() += 1;
            }
        }
        Ok(counts
            .into_iter()
            .map(|(slug, count)| TagWithCount {
                id: Self::deterministic_uuid(&["tag", slug]),
                slug: slug.to_string(),
                name: slug.to_string(),
                pinned: false,
                count,
            })
            .collect())
    }
}
