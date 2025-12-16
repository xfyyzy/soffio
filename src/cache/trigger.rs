//! Cache trigger service.
//!
//! Provides a high-level API for publishing cache events and optionally
//! consuming them immediately.

use std::sync::Arc;

use tracing::debug;
use uuid::Uuid;

use super::config::CacheConfig;
use super::consumer::CacheConsumer;
use super::events::{EventKind, EventQueue};

/// Cache trigger for publishing cache events.
///
/// This service wraps the event queue and consumer, providing convenience
/// methods for triggering cache invalidation from write operations.
///
/// # Usage
///
/// ```ignore
/// // After a successful post update:
/// trigger.post_upserted(post.id, &post.slug).await;
/// ```
pub struct CacheTrigger {
    config: CacheConfig,
    queue: Arc<EventQueue>,
    consumer: Arc<CacheConsumer>,
}

impl CacheTrigger {
    /// Create a new cache trigger.
    pub fn new(config: CacheConfig, queue: Arc<EventQueue>, consumer: Arc<CacheConsumer>) -> Self {
        Self {
            config,
            queue,
            consumer,
        }
    }

    /// Publish an event and optionally consume immediately.
    ///
    /// If `consume_now` is true, the consumer will be invoked immediately
    /// after publishing. Otherwise, events will be consumed by a background
    /// process or the next explicit consumption.
    pub async fn trigger(&self, kind: EventKind, consume_now: bool) {
        if !self.config.is_enabled() {
            debug!(event_kind = ?kind, "Cache trigger skipped: cache disabled");
            return;
        }

        self.queue.publish(kind);

        if consume_now {
            self.consumer.consume().await;
        }
    }

    /// Trigger a post upsert event (create or update).
    pub async fn post_upserted(&self, post_id: Uuid, slug: &str) {
        self.trigger(
            EventKind::PostUpserted {
                post_id,
                slug: slug.to_string(),
            },
            true,
        )
        .await;
    }

    /// Trigger a post delete event.
    pub async fn post_deleted(&self, post_id: Uuid, slug: &str) {
        self.trigger(
            EventKind::PostDeleted {
                post_id,
                slug: slug.to_string(),
            },
            true,
        )
        .await;
    }

    /// Trigger a page upsert event (create or update).
    pub async fn page_upserted(&self, page_id: Uuid, slug: &str) {
        self.trigger(
            EventKind::PageUpserted {
                page_id,
                slug: slug.to_string(),
            },
            true,
        )
        .await;
    }

    /// Trigger a page delete event.
    pub async fn page_deleted(&self, page_id: Uuid, slug: &str) {
        self.trigger(
            EventKind::PageDeleted {
                page_id,
                slug: slug.to_string(),
            },
            true,
        )
        .await;
    }

    /// Trigger a navigation update event.
    pub async fn navigation_updated(&self) {
        self.trigger(EventKind::NavigationUpdated, true).await;
    }

    /// Trigger a site settings update event.
    pub async fn site_settings_updated(&self) {
        self.trigger(EventKind::SiteSettingsUpdated, true).await;
    }

    /// Trigger an API key upsert event.
    pub async fn api_key_upserted(&self, prefix: &str) {
        self.trigger(
            EventKind::ApiKeyUpserted {
                prefix: prefix.to_string(),
            },
            true,
        )
        .await;
    }

    /// Trigger an API key revocation event.
    pub async fn api_key_revoked(&self, prefix: &str) {
        self.trigger(
            EventKind::ApiKeyRevoked {
                prefix: prefix.to_string(),
            },
            true,
        )
        .await;
    }

    /// Trigger a warmup event on application startup.
    pub async fn warmup_on_startup(&self) {
        self.trigger(EventKind::WarmupOnStartup, true).await;
    }

    /// Get the underlying config.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Get the underlying event queue.
    pub fn queue(&self) -> &Arc<EventQueue> {
        &self.queue
    }

    /// Get the underlying consumer.
    pub fn consumer(&self) -> &Arc<CacheConsumer> {
        &self.consumer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::registry::CacheRegistry;
    use crate::cache::store::{L0Store, L1Store};

    fn create_trigger() -> CacheTrigger {
        let config = CacheConfig::default();
        let l0 = Arc::new(L0Store::new(&config));
        let l1 = Arc::new(L1Store::new(&config));
        let registry = Arc::new(CacheRegistry::new());
        let queue = Arc::new(EventQueue::new());
        let consumer = Arc::new(CacheConsumer::new(
            config.clone(),
            l0,
            l1,
            registry,
            queue.clone(),
        ));

        CacheTrigger::new(config, queue, consumer)
    }

    fn create_disabled_trigger() -> CacheTrigger {
        let config = CacheConfig {
            enable_l0_cache: false,
            enable_l1_cache: false,
            ..Default::default()
        };
        let l0 = Arc::new(L0Store::new(&config));
        let l1 = Arc::new(L1Store::new(&config));
        let registry = Arc::new(CacheRegistry::new());
        let queue = Arc::new(EventQueue::new());
        let consumer = Arc::new(CacheConsumer::new(
            config.clone(),
            l0,
            l1,
            registry,
            queue.clone(),
        ));

        CacheTrigger::new(config, queue, consumer)
    }

    #[tokio::test]
    async fn trigger_publishes_event() {
        let trigger = create_trigger();

        // Before: queue is empty
        assert!(trigger.queue.is_empty());

        // Trigger without immediate consumption
        trigger.trigger(EventKind::SiteSettingsUpdated, false).await;

        // After: queue has one event (not consumed since consume_now=false)
        assert_eq!(trigger.queue.len(), 1);
    }

    #[tokio::test]
    async fn trigger_respects_disabled_config() {
        let trigger = create_disabled_trigger();

        trigger.post_upserted(Uuid::nil(), "test").await;

        // No events should be published when cache is disabled
        assert!(trigger.queue.is_empty());
    }

    #[tokio::test]
    async fn trigger_consumes_immediately_when_requested() {
        let trigger = create_trigger();

        trigger.site_settings_updated().await;

        // Event was published and consumed
        assert!(trigger.queue.is_empty());
    }

    #[tokio::test]
    async fn convenience_methods_work() {
        let trigger = create_trigger();

        trigger.post_upserted(Uuid::nil(), "post-slug").await;
        trigger.post_deleted(Uuid::nil(), "post-slug").await;
        trigger.page_upserted(Uuid::nil(), "page-slug").await;
        trigger.page_deleted(Uuid::nil(), "page-slug").await;
        trigger.navigation_updated().await;
        trigger.site_settings_updated().await;
        trigger.api_key_upserted("sof_").await;
        trigger.api_key_revoked("sof_").await;
        trigger.warmup_on_startup().await;

        // All events should have been consumed
        assert!(trigger.queue.is_empty());
    }
}
