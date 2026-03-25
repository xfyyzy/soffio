//! Cache storage implementations.
//!
//! L0: Object/query cache for domain entities.
//! L1: HTTP response cache for rendered pages.

use std::sync::RwLock;

use bytes::Bytes;
use lru::LruCache;
use metrics::counter;
use uuid::Uuid;

use crate::application::pagination::CursorPage;
use crate::application::repos::TagWithCount;
use crate::domain::api_keys::ApiKeyRecord;
use crate::domain::entities::{NavigationItemRecord, PageRecord, PostRecord, SiteSettingsRecord};
use crate::domain::posts::MonthCount;

use super::config::CacheConfig;
use super::keys::L1Key;
use super::lock::{rw_read, rw_write};

mod l0;
mod l1;

#[cfg(test)]
mod tests;

const SOURCE: &str = "cache::store";
const METRIC_L0_HIT_TOTAL: &str = "soffio_cache_l0_hit_total";
const METRIC_L0_MISS_TOTAL: &str = "soffio_cache_l0_miss_total";
const METRIC_L0_EVICT_TOTAL: &str = "soffio_cache_l0_evict_total";
const METRIC_L1_EVICT_TOTAL: &str = "soffio_cache_l1_evict_total";

fn record_l0_lookup(entity: &'static str, hit: bool) {
    if hit {
        counter!(METRIC_L0_HIT_TOTAL, "entity" => entity).increment(1);
    } else {
        counter!(METRIC_L0_MISS_TOTAL, "entity" => entity).increment(1);
    }
}

fn record_l0_evict(entity: &'static str) {
    counter!(METRIC_L0_EVICT_TOTAL, "entity" => entity).increment(1);
}

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
