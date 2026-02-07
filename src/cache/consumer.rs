//! Cache consumer for executing consumption plans.
//!
//! Consumes events from the queue and executes invalidation/warming actions.

use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use metrics::histogram;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::application::pagination::PageRequest;
use crate::application::repos::{
    NavigationQueryFilter, NavigationRepo, PagesRepo, PostListScope, PostQueryFilter, PostsRepo,
    SettingsRepo, TagsRepo,
};
use crate::infra::db::PostgresRepositories;

use super::config::CacheConfig;
use super::events::EventQueue;
use super::keys::{CacheKey, EntityKey};
use super::planner::ConsumptionPlan;
use super::registry::CacheRegistry;
use super::store::{L0Store, L1Store};

const METRIC_CACHE_CONSUME_MS: &str = "soffio_cache_consume_ms";
const METRIC_CACHE_WARM_MS: &str = "soffio_cache_warm_ms";

/// Cache consumer that processes events and maintains cache consistency.
///
/// The consumer:
/// 1. Drains events from the queue
/// 2. Generates a consumption plan from the events
/// 3. Executes the plan (invalidate L0, invalidate L1, warm)
pub struct CacheConsumer {
    config: CacheConfig,
    l0: Arc<L0Store>,
    l1: Arc<L1Store>,
    registry: Arc<CacheRegistry>,
    queue: Arc<EventQueue>,
    repos: Option<Arc<PostgresRepositories>>,
    #[cfg(test)]
    warm_invocations: Arc<AtomicUsize>,
}

impl CacheConsumer {
    /// Create a new cache consumer with repository access for warming.
    pub fn new(
        config: CacheConfig,
        l0: Arc<L0Store>,
        l1: Arc<L1Store>,
        registry: Arc<CacheRegistry>,
        queue: Arc<EventQueue>,
        repos: Arc<PostgresRepositories>,
    ) -> Self {
        Self {
            config,
            l0,
            l1,
            registry,
            queue,
            repos: Some(repos),
            #[cfg(test)]
            warm_invocations: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create a cache consumer without repository access (warming disabled).
    ///
    /// This is primarily for testing purposes.
    #[cfg(test)]
    pub fn new_without_repos(
        config: CacheConfig,
        l0: Arc<L0Store>,
        l1: Arc<L1Store>,
        registry: Arc<CacheRegistry>,
        queue: Arc<EventQueue>,
    ) -> Self {
        Self {
            config,
            l0,
            l1,
            registry,
            queue,
            repos: None,
            warm_invocations: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Consume pending events and execute the plan.
    ///
    /// Returns true if any events were processed.
    #[instrument(skip(self))]
    pub async fn consume(&self) -> bool {
        self.consume_with_mode(true).await
    }

    /// Consume pending events and run only invalidation actions.
    ///
    /// Useful on latency-sensitive write paths where pre-warming is deferred.
    #[instrument(skip(self))]
    pub async fn consume_invalidate_only(&self) -> bool {
        self.consume_with_mode(false).await
    }

    /// Consume pending events and run both invalidation and warming actions.
    #[instrument(skip(self))]
    pub async fn consume_full(&self) -> bool {
        self.consume_with_mode(true).await
    }

    async fn consume_with_mode(&self, include_warm: bool) -> bool {
        let consume_started_at = Instant::now();
        let events = self.queue.drain(self.config.consume_batch_limit);
        if events.is_empty() {
            return false;
        }

        let event_count = events.len();
        let event_ids: Vec<Uuid> = events.iter().map(|e| e.id).collect();
        let plan = ConsumptionPlan::from_events(events);

        // Observable: log consumption start with plan details
        info!(
            event_count,
            event_ids = ?event_ids,
            plan = %plan,
            include_warm,
            "Cache consumption starting"
        );

        // Phase 1: Invalidate L0 (skip if disabled or no entities to invalidate)
        if self.config.enable_l0_cache && !plan.invalidate_entities.is_empty() {
            self.invalidate_l0(&plan);
        }

        // Phase 2: Invalidate L1 using registry (skip if no entities to invalidate)
        if !plan.invalidate_entities.is_empty() {
            self.invalidate_l1(&plan);
        }

        // Phase 3: Warm cache from repositories (skip if L0 disabled or no warm actions)
        if include_warm && self.config.enable_l0_cache && plan.has_warm_actions() {
            self.warm(&plan).await;
        }

        // Observable: log consumption complete
        info!(
            event_count,
            invalidated = plan.invalidate_entities.len(),
            "Cache consumption complete"
        );

        histogram!(
            METRIC_CACHE_CONSUME_MS,
            "mode" => if include_warm { "full" } else { "invalidate_only" }
        )
        .record(consume_started_at.elapsed().as_secs_f64() * 1000.0);

        true
    }

    /// Invalidate L0 cache entries based on the plan.
    fn invalidate_l0(&self, plan: &ConsumptionPlan) {
        for entity in &plan.invalidate_entities {
            match entity {
                EntityKey::SiteSettings => self.l0.invalidate_site_settings(),
                EntityKey::Navigation => self.l0.invalidate_navigation(),
                EntityKey::Post(id) => {
                    // Try to get the post to know its slug
                    if let Some(post) = self.l0.get_post_by_id(*id) {
                        self.l0.invalidate_post(*id, &post.slug);
                    }
                }
                EntityKey::PostSlug(slug) => {
                    if let Some(post) = self.l0.get_post_by_slug(slug) {
                        self.l0.invalidate_post(post.id, slug);
                    }
                }
                EntityKey::Page(id) => {
                    if let Some(page) = self.l0.get_page_by_id(*id) {
                        self.l0.invalidate_page(*id, &page.slug);
                    }
                }
                EntityKey::PageSlug(slug) => {
                    if let Some(page) = self.l0.get_page_by_slug(slug) {
                        self.l0.invalidate_page(page.id, slug);
                    }
                }
                EntityKey::ApiKey(prefix) => {
                    self.l0.invalidate_api_key(prefix);
                }
                EntityKey::PostsIndex => self.l0.invalidate_all_post_lists(),
                EntityKey::PostAggTags => self.l0.invalidate_tag_counts(),
                EntityKey::PostAggMonths => self.l0.invalidate_month_counts(),
                EntityKey::Feed | EntityKey::Sitemap => {
                    // These are L1-only, handled in invalidate_l1
                }
            }
        }
    }

    /// Invalidate L1 cache entries based on the plan.
    fn invalidate_l1(&self, plan: &ConsumptionPlan) {
        for entity in &plan.invalidate_entities {
            let keys = self.registry.keys_for_entity(entity);
            for key in keys {
                if let CacheKey::L1(l1_key) = &key {
                    self.l1.invalidate(l1_key);
                }
                self.registry.unregister(&key);
            }
        }
    }

    /// Warm the cache based on the plan.
    ///
    /// Loads data from repositories and populates the L0 cache.
    /// Skipped if repository access is not available.
    async fn warm(&self, plan: &ConsumptionPlan) {
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

    /// Get reference to the event queue.
    pub fn queue(&self) -> &Arc<EventQueue> {
        &self.queue
    }

    /// Get reference to the L0 store.
    pub fn l0(&self) -> &Arc<L0Store> {
        &self.l0
    }

    /// Get reference to the L1 store.
    pub fn l1(&self) -> &Arc<L1Store> {
        &self.l1
    }

    /// Get reference to the registry.
    pub fn registry(&self) -> &Arc<CacheRegistry> {
        &self.registry
    }

    #[cfg(test)]
    fn warm_invocation_count(&self) -> usize {
        self.warm_invocations.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::events::EventKind;

    fn create_consumer() -> CacheConsumer {
        let config = CacheConfig::default();
        let l0 = Arc::new(L0Store::new(&config));
        let l1 = Arc::new(L1Store::new(&config));
        let registry = Arc::new(CacheRegistry::new());
        let queue = Arc::new(EventQueue::new());

        CacheConsumer::new_without_repos(config, l0, l1, registry, queue)
    }

    #[tokio::test]
    async fn consume_empty_queue_returns_false() {
        let consumer = create_consumer();
        assert!(!consumer.consume().await);
    }

    #[tokio::test]
    async fn consume_processes_events() {
        let consumer = create_consumer();

        consumer.queue.publish(EventKind::SiteSettingsUpdated);
        consumer.queue.publish(EventKind::NavigationUpdated);

        assert_eq!(consumer.queue.len(), 2);
        assert!(consumer.consume().await);
        assert!(consumer.queue.is_empty());
    }

    #[tokio::test]
    async fn consume_respects_batch_limit() {
        let config = CacheConfig {
            consume_batch_limit: 2,
            ..Default::default()
        };
        let l0 = Arc::new(L0Store::new(&config));
        let l1 = Arc::new(L1Store::new(&config));
        let registry = Arc::new(CacheRegistry::new());
        let queue = Arc::new(EventQueue::new());

        let consumer = CacheConsumer::new_without_repos(config, l0, l1, registry, queue);

        // Add 5 events
        for _ in 0..5 {
            consumer.queue.publish(EventKind::SiteSettingsUpdated);
        }

        assert_eq!(consumer.queue.len(), 5);
        consumer.consume().await;
        assert_eq!(consumer.queue.len(), 3); // Only consumed 2
    }

    #[tokio::test]
    async fn consume_invalidate_only_skips_warm_phase() {
        let consumer = create_consumer();

        consumer.queue.publish(EventKind::WarmupOnStartup);
        assert!(consumer.consume_invalidate_only().await);
        assert_eq!(consumer.warm_invocation_count(), 0);

        consumer.queue.publish(EventKind::WarmupOnStartup);
        assert!(consumer.consume_full().await);
        assert_eq!(consumer.warm_invocation_count(), 1);
    }

    #[tokio::test]
    async fn invalidate_l0_site_settings() {
        let consumer = create_consumer();

        // Cache something
        use crate::domain::entities::SiteSettingsRecord;
        use time::OffsetDateTime;

        let settings = SiteSettingsRecord {
            homepage_size: 10,
            admin_page_size: 20,
            show_tag_aggregations: true,
            show_month_aggregations: true,
            tag_filter_limit: 10,
            month_filter_limit: 12,
            global_toc_enabled: false,
            brand_title: "Test".to_string(),
            brand_href: "/".to_string(),
            footer_copy: "© 2024".to_string(),
            public_site_url: "http://localhost".to_string(),
            favicon_svg: "".to_string(),
            timezone: chrono_tz::Tz::UTC,
            meta_title: "Test".to_string(),
            meta_description: "Test".to_string(),
            og_title: "Test".to_string(),
            og_description: "Test".to_string(),
            updated_at: OffsetDateTime::now_utc(),
        };
        consumer.l0.set_site_settings(settings);
        assert!(consumer.l0.get_site_settings().is_some());

        // Publish invalidation event
        consumer.queue.publish(EventKind::SiteSettingsUpdated);
        consumer.consume().await;

        // Should be invalidated
        assert!(consumer.l0.get_site_settings().is_none());
    }
}
