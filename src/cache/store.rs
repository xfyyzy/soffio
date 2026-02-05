//! Cache storage implementations.
//!
//! L0: Object/query cache for domain entities.
//! L1: HTTP response cache for rendered pages.

use std::sync::RwLock;

use bytes::Bytes;
use lru::LruCache;
use uuid::Uuid;

use crate::application::pagination::CursorPage;
use crate::application::repos::TagWithCount;
use crate::domain::api_keys::ApiKeyRecord;
use crate::domain::entities::{NavigationItemRecord, PageRecord, PostRecord, SiteSettingsRecord};
use crate::domain::posts::MonthCount;

use super::config::CacheConfig;
use super::keys::L1Key;
use super::lock::{rw_read, rw_write};

const SOURCE: &str = "cache::store";

// ============================================================================
// L0 Store: Object/Query Cache
// ============================================================================

/// L0 object/query cache storage.
///
/// Provides in-memory caching for domain entities and query results.
/// Uses LRU eviction with configurable limits.
pub struct L0Store {
    // Singletons (no eviction needed)
    site_settings: RwLock<Option<SiteSettingsRecord>>,
    navigation: RwLock<Option<Vec<NavigationItemRecord>>>,
    tag_counts: RwLock<Option<Vec<TagWithCount>>>,
    month_counts: RwLock<Option<Vec<MonthCount>>>,

    // KV caches (with LRU eviction)
    posts_by_id: RwLock<LruCache<Uuid, PostRecord>>,
    posts_by_slug: RwLock<LruCache<String, PostRecord>>,
    pages_by_id: RwLock<LruCache<Uuid, PageRecord>>,
    pages_by_slug: RwLock<LruCache<String, PageRecord>>,
    api_keys_by_prefix: RwLock<LruCache<String, ApiKeyRecord>>,

    // List cache (high cardinality, strict LRU)
    // Key: (filter_hash, cursor_hash)
    post_lists: RwLock<LruCache<(u64, u64), CursorPage<PostRecord>>>,
}

impl L0Store {
    /// Create a new L0 store with the given configuration.
    pub fn new(config: &CacheConfig) -> Self {
        Self {
            site_settings: RwLock::new(None),
            navigation: RwLock::new(None),
            tag_counts: RwLock::new(None),
            month_counts: RwLock::new(None),
            posts_by_id: RwLock::new(LruCache::new(config.l0_post_limit_non_zero())),
            posts_by_slug: RwLock::new(LruCache::new(config.l0_post_limit_non_zero())),
            pages_by_id: RwLock::new(LruCache::new(config.l0_page_limit_non_zero())),
            pages_by_slug: RwLock::new(LruCache::new(config.l0_page_limit_non_zero())),
            api_keys_by_prefix: RwLock::new(LruCache::new(config.l0_api_key_limit_non_zero())),
            post_lists: RwLock::new(LruCache::new(config.l0_post_list_limit_non_zero())),
        }
    }

    // ========================================================================
    // Singleton getters/setters
    // ========================================================================

    pub fn get_site_settings(&self) -> Option<SiteSettingsRecord> {
        rw_read(&self.site_settings, SOURCE, "get_site_settings").clone()
    }

    pub fn set_site_settings(&self, value: SiteSettingsRecord) {
        *rw_write(&self.site_settings, SOURCE, "set_site_settings") = Some(value);
    }

    pub fn invalidate_site_settings(&self) {
        *rw_write(&self.site_settings, SOURCE, "invalidate_site_settings") = None;
    }

    pub fn get_navigation(&self) -> Option<Vec<NavigationItemRecord>> {
        rw_read(&self.navigation, SOURCE, "get_navigation").clone()
    }

    pub fn set_navigation(&self, value: Vec<NavigationItemRecord>) {
        *rw_write(&self.navigation, SOURCE, "set_navigation") = Some(value);
    }

    pub fn invalidate_navigation(&self) {
        *rw_write(&self.navigation, SOURCE, "invalidate_navigation") = None;
    }

    pub fn get_tag_counts(&self) -> Option<Vec<TagWithCount>> {
        rw_read(&self.tag_counts, SOURCE, "get_tag_counts").clone()
    }

    pub fn set_tag_counts(&self, value: Vec<TagWithCount>) {
        *rw_write(&self.tag_counts, SOURCE, "set_tag_counts") = Some(value);
    }

    pub fn invalidate_tag_counts(&self) {
        *rw_write(&self.tag_counts, SOURCE, "invalidate_tag_counts") = None;
    }

    pub fn get_month_counts(&self) -> Option<Vec<MonthCount>> {
        rw_read(&self.month_counts, SOURCE, "get_month_counts").clone()
    }

    pub fn set_month_counts(&self, value: Vec<MonthCount>) {
        *rw_write(&self.month_counts, SOURCE, "set_month_counts") = Some(value);
    }

    pub fn invalidate_month_counts(&self) {
        *rw_write(&self.month_counts, SOURCE, "invalidate_month_counts") = None;
    }

    // ========================================================================
    // Post KV cache
    // ========================================================================

    pub fn get_post_by_id(&self, id: Uuid) -> Option<PostRecord> {
        rw_write(&self.posts_by_id, SOURCE, "get_post_by_id")
            .get(&id)
            .cloned()
    }

    pub fn get_post_by_slug(&self, slug: &str) -> Option<PostRecord> {
        rw_write(&self.posts_by_slug, SOURCE, "get_post_by_slug")
            .get(slug)
            .cloned()
    }

    pub fn set_post(&self, post: PostRecord) {
        let mut by_id = rw_write(&self.posts_by_id, SOURCE, "set_post.by_id");
        let mut by_slug = rw_write(&self.posts_by_slug, SOURCE, "set_post.by_slug");
        by_id.put(post.id, post.clone());
        by_slug.put(post.slug.clone(), post);
    }

    pub fn invalidate_post(&self, id: Uuid, slug: &str) {
        rw_write(&self.posts_by_id, SOURCE, "invalidate_post.by_id").pop(&id);
        rw_write(&self.posts_by_slug, SOURCE, "invalidate_post.by_slug").pop(slug);
    }

    // ========================================================================
    // Page KV cache
    // ========================================================================

    pub fn get_page_by_id(&self, id: Uuid) -> Option<PageRecord> {
        rw_write(&self.pages_by_id, SOURCE, "get_page_by_id")
            .get(&id)
            .cloned()
    }

    pub fn get_page_by_slug(&self, slug: &str) -> Option<PageRecord> {
        rw_write(&self.pages_by_slug, SOURCE, "get_page_by_slug")
            .get(slug)
            .cloned()
    }

    pub fn set_page(&self, page: PageRecord) {
        let mut by_id = rw_write(&self.pages_by_id, SOURCE, "set_page.by_id");
        let mut by_slug = rw_write(&self.pages_by_slug, SOURCE, "set_page.by_slug");
        by_id.put(page.id, page.clone());
        by_slug.put(page.slug.clone(), page);
    }

    pub fn invalidate_page(&self, id: Uuid, slug: &str) {
        rw_write(&self.pages_by_id, SOURCE, "invalidate_page.by_id").pop(&id);
        rw_write(&self.pages_by_slug, SOURCE, "invalidate_page.by_slug").pop(slug);
    }

    // ========================================================================
    // API key KV cache
    // ========================================================================

    pub fn get_api_key_by_prefix(&self, prefix: &str) -> Option<ApiKeyRecord> {
        rw_write(&self.api_keys_by_prefix, SOURCE, "get_api_key_by_prefix")
            .get(prefix)
            .cloned()
    }

    pub fn set_api_key(&self, key: ApiKeyRecord) {
        rw_write(&self.api_keys_by_prefix, SOURCE, "set_api_key").put(key.prefix.clone(), key);
    }

    pub fn invalidate_api_key(&self, prefix: &str) {
        rw_write(&self.api_keys_by_prefix, SOURCE, "invalidate_api_key").pop(prefix);
    }

    // ========================================================================
    // Post list cache
    // ========================================================================

    pub fn get_post_list(
        &self,
        filter_hash: u64,
        cursor_hash: u64,
    ) -> Option<CursorPage<PostRecord>> {
        rw_write(&self.post_lists, SOURCE, "get_post_list")
            .get(&(filter_hash, cursor_hash))
            .cloned()
    }

    pub fn set_post_list(&self, filter_hash: u64, cursor_hash: u64, page: CursorPage<PostRecord>) {
        rw_write(&self.post_lists, SOURCE, "set_post_list").put((filter_hash, cursor_hash), page);
    }

    pub fn invalidate_all_post_lists(&self) {
        rw_write(&self.post_lists, SOURCE, "invalidate_all_post_lists").clear();
    }

    // ========================================================================
    // Bulk operations
    // ========================================================================

    /// Clear all cached data.
    pub fn clear(&self) {
        self.invalidate_site_settings();
        self.invalidate_navigation();
        self.invalidate_tag_counts();
        self.invalidate_month_counts();
        rw_write(&self.posts_by_id, SOURCE, "clear.posts_by_id").clear();
        rw_write(&self.posts_by_slug, SOURCE, "clear.posts_by_slug").clear();
        rw_write(&self.pages_by_id, SOURCE, "clear.pages_by_id").clear();
        rw_write(&self.pages_by_slug, SOURCE, "clear.pages_by_slug").clear();
        rw_write(&self.api_keys_by_prefix, SOURCE, "clear.api_keys_by_prefix").clear();
        rw_write(&self.post_lists, SOURCE, "clear.post_lists").clear();
    }
}

// ============================================================================
// L1 Store: Response Cache
// ============================================================================

/// Cached HTTP response.
#[derive(Clone)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
}

/// L1 response cache storage.
///
/// Caches rendered HTTP responses for public pages.
pub struct L1Store {
    responses: RwLock<LruCache<L1Key, CachedResponse>>,
}

impl L1Store {
    /// Create a new L1 store with the given configuration.
    pub fn new(config: &CacheConfig) -> Self {
        Self {
            responses: RwLock::new(LruCache::new(config.l1_response_limit_non_zero())),
        }
    }

    pub fn get(&self, key: &L1Key) -> Option<CachedResponse> {
        rw_write(&self.responses, SOURCE, "l1_get")
            .get(key)
            .cloned()
    }

    pub fn set(&self, key: L1Key, response: CachedResponse) -> Option<L1Key> {
        rw_write(&self.responses, SOURCE, "l1_set")
            .push(key, response)
            .map(|(evicted_key, _)| evicted_key)
    }

    pub fn invalidate(&self, key: &L1Key) {
        rw_write(&self.responses, SOURCE, "l1_invalidate").pop(key);
    }

    pub fn invalidate_all(&self) {
        rw_write(&self.responses, SOURCE, "l1_invalidate_all").clear();
    }

    /// Get the number of cached responses.
    pub fn len(&self) -> usize {
        rw_read(&self.responses, SOURCE, "l1_len").len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use time::OffsetDateTime;

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
}
