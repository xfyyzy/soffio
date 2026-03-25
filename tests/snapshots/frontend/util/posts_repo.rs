use super::*;

#[async_trait]
impl PostsRepo for StaticContentRepo {
    async fn list_posts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
        page: PageRequest<PostCursor>,
    ) -> Result<CursorPage<PostRecord>, RepoError> {
        let posts = self.sorted_posts(scope, filter);
        let limit = page.limit.clamp(1, 100) as usize;

        let start = page
            .cursor
            .as_ref()
            .and_then(|cursor| {
                posts
                    .iter()
                    .position(|post| Self::post_uuid(post.slug) == cursor.id())
                    .map(|idx| idx + 1)
            })
            .unwrap_or(0);

        let slice = posts
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let records: Vec<PostRecord> = slice.iter().map(|post| Self::record_for(post)).collect();

        let next_cursor = posts.get(start + limit).map(|post| {
            PostCursor::published(
                post.date.with_time(time!(00:00:00)).assume_utc(),
                Self::post_uuid(post.slug),
                false,
            )
            .encode()
        });

        Ok(CursorPage::new(records, next_cursor))
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<PostRecord>, RepoError> {
        Ok(posts::find_by_slug(slug).map(Self::record_for))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<PostRecord>, RepoError> {
        Ok(self
            .all_posts()
            .into_iter()
            .find(|post| Self::post_uuid(post.slug) == id)
            .map(Self::record_for))
    }

    async fn list_month_counts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<Vec<posts::MonthCount>, RepoError> {
        let posts: Vec<&posts::Post> = self.sorted_posts(scope, filter);

        let mut counts: BTreeMap<String, (String, usize)> = BTreeMap::new();
        for post in posts {
            let key = posts::month_key_for(post.date);
            let label = posts::month_label_for(post.date);
            counts
                .entry(key)
                .and_modify(|entry| entry.1 += 1)
                .or_insert((label, 1));
        }

        let mut months = counts
            .into_iter()
            .map(|(key, (label, count))| posts::MonthCount { key, label, count })
            .collect::<Vec<_>>();
        months.sort_by(|a, b| b.key.cmp(&a.key));
        Ok(months)
    }

    async fn count_posts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<u64, RepoError> {
        let posts = self.sorted_posts(scope, filter);
        Ok(posts.len() as u64)
    }

    async fn count_posts_before(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
        cursor: &PostCursor,
    ) -> Result<u64, RepoError> {
        let posts = self.sorted_posts(scope, filter);
        let idx = posts
            .iter()
            .position(|post| Self::post_uuid(post.slug) == cursor.id())
            .unwrap_or(0);
        Ok(idx as u64)
    }

    async fn list_tag_counts(
        &self,
        scope: PostListScope,
        filter: &PostQueryFilter,
    ) -> Result<Vec<PostTagCount>, RepoError> {
        let posts = self.sorted_posts(scope, filter);
        let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
        for post in posts {
            for tag in post.tags {
                *counts.entry(tag).or_default() += 1;
            }
        }
        Ok(counts
            .into_iter()
            .map(|(slug, count)| PostTagCount {
                slug: slug.to_string(),
                name: slug.to_string(),
                count,
            })
            .collect())
    }
}
