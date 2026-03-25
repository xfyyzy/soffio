use std::sync::Arc;

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
