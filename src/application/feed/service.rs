use super::*;

impl FeedService {
    pub fn new(
        posts: Arc<dyn PostsRepo>,
        sections: Arc<dyn SectionsRepo>,
        tags: Arc<dyn TagsRepo>,
        settings: Arc<dyn SettingsRepo>,
        cache: Option<Arc<L0Store>>,
    ) -> Self {
        Self {
            posts,
            sections,
            tags,
            settings,
            cache,
        }
    }

    fn decode_cursor(&self, cursor: Option<&str>) -> Result<Option<PostCursor>, FeedError> {
        cursor
            .map(PostCursor::decode)
            .transpose()
            .map_err(|err| FeedError::InvalidCursor(err.to_string()))
    }

    pub async fn page_context(
        &self,
        filter: FeedFilter,
        cursor: Option<&str>,
    ) -> Result<PageContext, FeedError> {
        // Record derived collection dependencies for cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::PostsIndex);
        crate::cache::deps::record(crate::cache::EntityKey::PostAggTags);
        crate::cache::deps::record(crate::cache::EntityKey::PostAggMonths);

        let decoded_cursor = self.decode_cursor(cursor)?;
        let query_filter = filter.to_query_filter();
        let settings = self.load_site_settings().await?;
        let page_limit = presentation::homepage_page_limit(&settings);

        let filter_hash = hash_post_list_key(&query_filter, page_limit);
        let cursor_hash = hash_cursor_str(cursor);

        let page = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_post_list(filter_hash, cursor_hash) {
                cached
            } else {
                let page = self
                    .posts
                    .list_posts(
                        PostListScope::Public,
                        &query_filter,
                        PageRequest::new(page_limit, decoded_cursor),
                    )
                    .await?;
                cache.set_post_list(filter_hash, cursor_hash, page.clone());
                page
            }
        } else {
            self.posts
                .list_posts(
                    PostListScope::Public,
                    &query_filter,
                    PageRequest::new(page_limit, decoded_cursor),
                )
                .await?
        };

        let total_filtered = self
            .posts
            .count_posts(PostListScope::Public, &query_filter)
            .await?;

        let total_all = self
            .posts
            .count_posts(PostListScope::Public, &PostQueryFilter::default())
            .await?;

        let tag_counts = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_tag_counts() {
                cached
            } else {
                let tags = self.tags.list_with_counts().await?;
                cache.set_tag_counts(tags.clone());
                tags
            }
        } else {
            self.tags.list_with_counts().await?
        };
        let month_counts = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_month_counts() {
                cached
            } else {
                let months = self
                    .posts
                    .list_month_counts(PostListScope::Public, &PostQueryFilter::default())
                    .await?;
                cache.set_month_counts(months.clone());
                months
            }
        } else {
            self.posts
                .list_month_counts(PostListScope::Public, &PostQueryFilter::default())
                .await?
        };

        let tag_summaries = if settings.show_tag_aggregations {
            summaries::build_tag_summaries(&tag_counts, filter.tag(), total_all, &settings)
        } else {
            Vec::new()
        };

        let month_summaries = if settings.show_month_aggregations {
            summaries::build_month_summaries(
                &month_counts,
                filter.month(),
                total_all,
                settings.month_filter_limit,
            )
        } else {
            Vec::new()
        };

        let mut cards = Vec::with_capacity(page.items.len());
        for record in &page.items {
            let tags = self.tags.list_for_post(record.id).await?;
            cards.push(presentation::record_to_card(
                record,
                &tags,
                settings.timezone,
            ));
        }

        let posts_ld_json = presentation::build_posts_ld_json(
            &cards,
            &filter,
            &settings.public_site_url,
            &settings.meta_title,
        );

        let post_count = cards.len();
        Ok(PageContext {
            posts: cards,
            post_count,
            total_count: usize::try_from(total_filtered).unwrap_or(usize::MAX),
            has_results: post_count > 0,
            tags: tag_summaries,
            months: month_summaries,
            show_tag_filters: settings.show_tag_aggregations,
            show_month_filters: settings.show_month_aggregations,
            next_cursor: page.next_cursor,
            load_more_query: filter.load_more_query(),
            posts_ld_json,
        })
    }

    pub async fn append_payload(
        &self,
        filter: FeedFilter,
        cursor: Option<&str>,
    ) -> Result<AppendPayload, FeedError> {
        let decoded_cursor = self.decode_cursor(cursor)?;
        let query_filter = filter.to_query_filter();
        let settings = self.load_site_settings().await?;
        let page_limit = presentation::homepage_page_limit(&settings);
        let filter_hash = hash_post_list_key(&query_filter, page_limit);
        let cursor_hash = hash_cursor_str(cursor);

        let page = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_post_list(filter_hash, cursor_hash) {
                cached
            } else {
                let page = self
                    .posts
                    .list_posts(
                        PostListScope::Public,
                        &query_filter,
                        PageRequest::new(page_limit, decoded_cursor),
                    )
                    .await?;
                cache.set_post_list(filter_hash, cursor_hash, page.clone());
                page
            }
        } else {
            self.posts
                .list_posts(
                    PostListScope::Public,
                    &query_filter,
                    PageRequest::new(page_limit, decoded_cursor),
                )
                .await?
        };

        let mut cards = Vec::with_capacity(page.items.len());
        for record in &page.items {
            let tags = self.tags.list_for_post(record.id).await?;
            cards.push(presentation::record_to_card(
                record,
                &tags,
                settings.timezone,
            ));
        }

        let offset = if let Some(cursor) = decoded_cursor {
            self.posts
                .count_posts_before(PostListScope::Public, &query_filter, &cursor)
                .await?
        } else {
            0
        };

        let offset_usize = usize::try_from(offset).unwrap_or(usize::MAX);
        let total_visible = offset_usize.saturating_add(cards.len());

        Ok(AppendPayload {
            offset: offset_usize,
            cards,
            next_cursor: page.next_cursor,
            total_visible,
        })
    }

    pub async fn post_detail(&self, slug: &str) -> Result<Option<PostDetailContext>, FeedError> {
        // Record post slug dependency for cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::PostSlug(slug.to_string()));

        let post = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_post_by_slug(slug) {
                Some(cached)
            } else {
                let fetched = self.posts.find_by_slug(slug).await?;
                if let Some(post) = fetched.clone() {
                    cache.set_post(post);
                }
                fetched
            }
        } else {
            self.posts.find_by_slug(slug).await?
        };

        let Some(post) = post else {
            return Ok(None);
        };

        if post.status != PostStatus::Published || post.published_at.is_none() {
            return Ok(None);
        }

        self.build_post_context(post).await.map(Some)
    }

    pub async fn post_preview(&self, id: Uuid) -> Result<Option<PostDetailContext>, FeedError> {
        let Some(post) = self.posts.find_by_id(id).await? else {
            return Ok(None);
        };

        self.build_post_context(post).await.map(Some)
    }

    async fn build_post_context(&self, post: PostRecord) -> Result<PostDetailContext, FeedError> {
        let sections = self.sections.list_sections(post.id).await?;
        let section_nodes = build_section_tree(sections)?;
        let tags = self.tags.list_for_post(post.id).await?;
        let settings = self.load_site_settings().await?;

        let has_code_blocks = PostSectionNode::any_contains_code(&section_nodes);
        let has_math_blocks = PostSectionNode::any_contains_math(&section_nodes);
        let has_mermaid_diagrams = PostSectionNode::any_contains_mermaid(&section_nodes);
        let sections = sections::build_post_section_events(&section_nodes);
        let toc = if settings.global_toc_enabled {
            sections::build_post_toc_view(&section_nodes)
        } else {
            None
        };

        let published_at = post.published_at.unwrap_or(post.created_at);
        let localized = timezone::localized_datetime(published_at, settings.timezone);
        let date = timezone::localized_date(published_at, settings.timezone);

        Ok(PostDetailContext {
            slug: post.slug,
            title: post.title,
            published: posts::format_human_date(date),
            iso_date: localized.to_rfc3339(),
            tags: build_tag_badges(
                tags.iter()
                    .map(|tag| (tag.slug.as_str(), tag.name.as_str())),
            ),
            excerpt: post.excerpt,
            summary_html: post.summary_html,
            sections,
            has_code_blocks,
            has_math_blocks,
            has_mermaid_diagrams,
            toc,
            is_pinned: post.pinned,
        })
    }

    pub async fn is_known_tag(&self, tag: &str) -> Result<bool, FeedError> {
        crate::cache::deps::record(crate::cache::EntityKey::PostAggTags);

        let tags = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_tag_counts() {
                cached
            } else {
                let tags = self.tags.list_with_counts().await?;
                cache.set_tag_counts(tags.clone());
                tags
            }
        } else {
            self.tags.list_with_counts().await?
        };
        Ok(tags.iter().any(|record| record.slug == tag))
    }

    pub async fn is_known_month(&self, month: &str) -> Result<bool, FeedError> {
        crate::cache::deps::record(crate::cache::EntityKey::PostAggMonths);

        let months = if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get_month_counts() {
                cached
            } else {
                let months = self
                    .posts
                    .list_month_counts(PostListScope::Public, &PostQueryFilter::default())
                    .await?;
                cache.set_month_counts(months.clone());
                months
            }
        } else {
            self.posts
                .list_month_counts(PostListScope::Public, &PostQueryFilter::default())
                .await?
        };
        Ok(months.iter().any(|entry| entry.key == month))
    }

    async fn load_site_settings(&self) -> Result<SiteSettingsRecord, FeedError> {
        // Record site settings dependency for cache invalidation
        crate::cache::deps::record(crate::cache::EntityKey::SiteSettings);

        if let Some(settings) = self
            .cache
            .as_ref()
            .and_then(|cache| cache.get_site_settings())
        {
            return Ok(settings);
        }

        let settings = self
            .settings
            .load_site_settings()
            .await
            .map_err(FeedError::from)?;

        if let Some(cache) = &self.cache {
            cache.set_site_settings(settings.clone());
        }

        Ok(settings)
    }
}
