use std::panic::{AssertUnwindSafe, catch_unwind};

use time::OffsetDateTime;
use uuid::Uuid;

use super::*;

fn sample_post(id: Uuid, slug: &str) -> PostRecord {
    use crate::domain::types::PostStatus;
    PostRecord {
        id,
        slug: slug.to_string(),
        title: "Test Post".to_string(),
        excerpt: "".to_string(),
        body_markdown: "".to_string(),
        status: PostStatus::Published,
        pinned: false,
        scheduled_at: None,
        published_at: Some(OffsetDateTime::now_utc()),
        archived_at: None,
        summary_markdown: None,
        summary_html: None,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    }
}

fn sample_settings() -> SiteSettingsRecord {
    SiteSettingsRecord {
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
        meta_title: "Test Site".to_string(),
        meta_description: "Test description".to_string(),
        og_title: "Test Site".to_string(),
        og_description: "Test OG description".to_string(),
        updated_at: OffsetDateTime::now_utc(),
    }
}

#[test]
fn l0_post_cache_roundtrip() {
    let config = CacheConfig::default();
    let store = L0Store::new(&config);

    let id = Uuid::new_v4();
    let post = sample_post(id, "test-post");

    assert!(store.get_post_by_id(id).is_none());

    store.set_post(post.clone());

    let cached = store.get_post_by_id(id).expect("cached post");
    assert_eq!(cached.slug, "test-post");

    let by_slug = store.get_post_by_slug("test-post").expect("cached by slug");
    assert_eq!(by_slug.id, id);

    store.invalidate_post(id, "test-post");

    assert!(store.get_post_by_id(id).is_none());
    assert!(store.get_post_by_slug("test-post").is_none());
}

#[test]
fn l0_singleton_cache() {
    let config = CacheConfig::default();
    let store = L0Store::new(&config);

    assert!(store.get_site_settings().is_none());

    let settings = sample_settings();

    store.set_site_settings(settings.clone());

    let cached = store.get_site_settings().expect("cached settings");
    assert_eq!(cached.brand_title, "Test");

    store.invalidate_site_settings();
    assert!(store.get_site_settings().is_none());
}

#[test]
fn l1_response_cache_roundtrip() {
    let config = CacheConfig::default();
    let store = L1Store::new(&config);

    use super::super::keys::OutputFormat;

    let key = L1Key::Response {
        format: OutputFormat::Html,
        path: "/posts/test".to_string(),
        query_hash: 0,
    };

    assert!(store.get(&key).is_none());

    let response = CachedResponse {
        status: 200,
        headers: vec![("Content-Type".to_string(), "text/html".to_string())],
        body: Bytes::from("Hello"),
    };

    let evicted = store.set(key.clone(), response);
    assert!(evicted.is_none());

    let cached = store.get(&key).expect("cached response");
    assert_eq!(cached.status, 200);
    assert_eq!(cached.body, Bytes::from("Hello"));

    store.invalidate(&key);
    assert!(store.get(&key).is_none());
}

#[test]
fn l0_lru_eviction() {
    let config = CacheConfig {
        l0_post_limit: 2,
        ..Default::default()
    };
    let store = L0Store::new(&config);

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let id3 = Uuid::new_v4();

    store.set_post(sample_post(id1, "post-1"));
    store.set_post(sample_post(id2, "post-2"));

    // Both should be cached
    assert!(store.get_post_by_id(id1).is_some());
    assert!(store.get_post_by_id(id2).is_some());

    // Adding third should evict first (LRU)
    store.set_post(sample_post(id3, "post-3"));

    assert!(store.get_post_by_id(id1).is_none()); // Evicted
    assert!(store.get_post_by_id(id2).is_some());
    assert!(store.get_post_by_id(id3).is_some());
}

#[test]
fn l0_store_recovers_from_poisoned_lock() {
    let config = CacheConfig::default();
    let store = L0Store::new(&config);

    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _guard = store
            .site_settings
            .write()
            .expect("site_settings lock should be acquired");
        panic!("poison site_settings lock");
    }));

    store.set_site_settings(sample_settings());
    assert!(store.get_site_settings().is_some());
}
