use super::*;

impl CacheConsumer {
    /// Warm the cache based on the plan.
    ///
    /// Loads data from repositories and populates the L0 cache.
    /// Skipped if repository access is not available.
    pub(super) async fn warm(&self, plan: &ConsumptionPlan) {
        let warm_started_at = Instant::now();
        #[cfg(test)]
        self.warm_invocations.fetch_add(1, Ordering::Relaxed);

        let Some(repos) = &self.repos else {
            tracing::debug!("Warming skipped: no repository access");
            histogram!(METRIC_CACHE_WARM_MS)
                .record(warm_started_at.elapsed().as_secs_f64() * 1000.0);
            return;
        };

        // Warm site settings
        if plan.warm_site_settings
            && let Ok(settings) = SettingsRepo::load_site_settings(repos.as_ref()).await
        {
            self.l0.set_site_settings(settings);
            tracing::debug!("Warmed: site settings");
        }

        // Warm navigation and optionally linked pages
        if plan.warm_navigation {
            let filter = NavigationQueryFilter::default();
            let page_req = PageRequest::new(100, None);
            if let Ok(page) = NavigationRepo::list_navigation(
                repos.as_ref(),
                Some(true), // visible only
                &filter,
                page_req,
            )
            .await
            {
                self.l0.set_navigation(page.items.clone());
                tracing::debug!(count = page.items.len(), "Warmed: navigation");

                // Warm pages linked from visible navigation
                if plan.warm_navigation_pages {
                    for item in &page.items {
                        if let Some(page_id) = item.destination_page_id
                            && let Ok(Some(page_record)) =
                                PagesRepo::find_by_id(repos.as_ref(), page_id).await
                        {
                            self.l0.set_page(page_record);
                        }
                    }
                    tracing::debug!("Warmed: navigation pages");
                }
            }
        }

        // Warm aggregations (tag counts, month counts)
        if plan.warm_aggregations {
            if let Ok(tags) = TagsRepo::list_with_counts(repos.as_ref()).await {
                self.l0.set_tag_counts(tags);
                tracing::debug!("Warmed: tag counts");
            }

            let filter = PostQueryFilter::default();
            if let Ok(months) =
                PostsRepo::list_month_counts(repos.as_ref(), PostListScope::Public, &filter).await
            {
                self.l0.set_month_counts(months);
                tracing::debug!("Warmed: month counts");
            }
        }

        // Warm individual posts
        for post_id in &plan.warm_posts {
            if let Ok(Some(post)) = PostsRepo::find_by_id(repos.as_ref(), *post_id).await {
                self.l0.set_post(post);
            }
        }
        if !plan.warm_posts.is_empty() {
            tracing::debug!(count = plan.warm_posts.len(), "Warmed: posts");
        }

        // Warm individual pages
        for page_id in &plan.warm_pages {
            if let Ok(Some(page)) = PagesRepo::find_by_id(repos.as_ref(), *page_id).await {
                self.l0.set_page(page);
            }
        }
        if !plan.warm_pages.is_empty() {
            tracing::debug!(count = plan.warm_pages.len(), "Warmed: pages");
        }

        // Warm homepage first page of posts
        if plan.warm_homepage {
            let filter = PostQueryFilter::default();
            let page_req = PageRequest::new(20, None); // First page
            if let Ok(page) =
                PostsRepo::list_posts(repos.as_ref(), PostListScope::Public, &filter, page_req)
                    .await
            {
                // Cache each post from the homepage
                for post in page.items {
                    self.l0.set_post(post);
                }
                tracing::debug!("Warmed: homepage posts");
            }
        }

        // Note: warm_feed and warm_sitemap are L1-only (HTTP response cache)
        // They will be populated on first request via read-through
        if plan.warm_feed {
            tracing::debug!("Feed warming deferred to first request (L1 only)");
        }
        if plan.warm_sitemap {
            tracing::debug!("Sitemap warming deferred to first request (L1 only)");
        }

        histogram!(METRIC_CACHE_WARM_MS).record(warm_started_at.elapsed().as_secs_f64() * 1000.0);
    }
}
