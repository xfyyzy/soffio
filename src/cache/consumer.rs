//! Cache consumer for executing consumption plans.
//!
//! Consumes events from the queue and executes invalidation/warming actions.

use std::sync::Arc;

use tracing::{info, instrument};
use uuid::Uuid;

use super::config::CacheConfig;
use super::events::EventQueue;
use super::keys::{CacheKey, EntityKey};
use super::planner::ConsumptionPlan;
use super::registry::CacheRegistry;
use super::store::{L0Store, L1Store};

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
}

impl CacheConsumer {
    /// Create a new cache consumer.
    pub fn new(
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
        }
    }

    /// Consume pending events and execute the plan.
    ///
    /// Returns true if any events were processed.
    #[instrument(skip(self))]
    pub async fn consume(&self) -> bool {
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
            "Cache consumption starting"
        );

        // Phase 1: Invalidate L0
        self.invalidate_l0(&plan);

        // Phase 2: Invalidate L1 using registry
        self.invalidate_l1(&plan);

        // Phase 3: Warm (placeholder for Phase 3 integration)
        // Note: Warming requires repository access which will be added in Phase 3
        self.warm(&plan).await;

        // Observable: log consumption complete
        info!(
            event_count,
            invalidated = plan.invalidate_entities.len(),
            "Cache consumption complete"
        );

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
    /// Note: This is a placeholder that will be implemented in Phase 3
    /// when repository access is integrated.
    async fn warm(&self, plan: &ConsumptionPlan) {
        // Phase 3 will implement actual warming by:
        // 1. Loading data from repositories
        // 2. Populating L0 cache

        // Log what would be warmed for observability
        if plan.warm_site_settings {
            tracing::debug!("Would warm: site settings");
        }
        if plan.warm_navigation {
            tracing::debug!("Would warm: navigation");
        }
        if plan.warm_navigation_pages {
            tracing::debug!("Would warm: navigation pages");
        }
        if plan.warm_aggregations {
            tracing::debug!("Would warm: aggregations");
        }
        if !plan.warm_posts.is_empty() {
            tracing::debug!(count = plan.warm_posts.len(), "Would warm: posts");
        }
        if !plan.warm_pages.is_empty() {
            tracing::debug!(count = plan.warm_pages.len(), "Would warm: pages");
        }
        if plan.warm_homepage {
            tracing::debug!("Would warm: homepage");
        }
        if plan.warm_feed {
            tracing::debug!("Would warm: feed");
        }
        if plan.warm_sitemap {
            tracing::debug!("Would warm: sitemap");
        }
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

        CacheConsumer::new(config, l0, l1, registry, queue)
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

        let consumer = CacheConsumer::new(config, l0, l1, registry, queue);

        // Add 5 events
        for _ in 0..5 {
            consumer.queue.publish(EventKind::SiteSettingsUpdated);
        }

        assert_eq!(consumer.queue.len(), 5);
        consumer.consume().await;
        assert_eq!(consumer.queue.len(), 3); // Only consumed 2
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
