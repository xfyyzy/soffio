//! Cache key definitions.
//!
//! Defines `EntityKey` for domain entities and `CacheKey` for cache entries.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use uuid::Uuid;

use crate::application::repos::PostQueryFilter;

/// Identifies a domain entity or derived collection for cache invalidation.
///
/// When an entity changes, all cache entries that depend on it must be invalidated.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityKey {
    // Singletons
    /// Site-wide settings (brand, metadata, etc.)
    SiteSettings,
    /// Navigation menu items
    Navigation,

    // Content entities (by ID for write, slug for read)
    /// A post identified by its database ID
    Post(Uuid),
    /// A post identified by its URL slug
    PostSlug(String),
    /// A page identified by its database ID
    Page(Uuid),
    /// A page identified by its URL slug
    PageSlug(String),

    // Security
    /// An API key identified by its prefix
    ApiKey(String),

    // Derived collections (invalidated when any post/page changes)
    /// Homepage, archives, tag/month filtered lists
    PostsIndex,
    /// Tag counts for sidebar
    PostAggTags,
    /// Month counts for sidebar
    PostAggMonths,
    /// RSS/Atom feed
    Feed,
    /// XML sitemap
    Sitemap,
}

/// Output format for L1 response cache.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    Html,
    Json,
    Rss,
    Atom,
    Sitemap,
    Favicon,
}

/// L0 object/query cache keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum L0Key {
    // Singletons
    SiteSettings,
    Navigation,
    TagCounts,
    MonthCounts,

    // KV lookups
    PostById(Uuid),
    PostBySlug(String),
    PageById(Uuid),
    PageBySlug(String),
    ApiKeyByPrefix(String),

    // LRU lists (keyed by filter hash + cursor)
    PostList { filter_hash: u64, cursor_hash: u64 },
}

/// L1 response cache keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum L1Key {
    Response {
        format: OutputFormat,
        path: String,
        query_hash: u64,
    },
}

/// Unified cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheKey {
    L0(L0Key),
    L1(L1Key),
}

// ============================================================================
// Hash Utilities
// ============================================================================

/// Compute a hash for any hashable value.
pub fn hash_value<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Hash a query string for L1 cache key generation.
pub fn hash_query(query: &str) -> u64 {
    hash_value(&query)
}

/// Hash a post list filter with page limit for L0 list cache keys.
pub fn hash_post_list_key(filter: &PostQueryFilter, page_limit: u32) -> u64 {
    let mut hasher = DefaultHasher::new();
    filter.tag.hash(&mut hasher);
    filter.month.hash(&mut hasher);
    filter.search.hash(&mut hasher);
    page_limit.hash(&mut hasher);
    hasher.finish()
}

/// Hash an optional cursor string for L0 list cache keys.
pub fn hash_cursor_str(cursor: Option<&str>) -> u64 {
    hash_value(&cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_key_equality() {
        let key1 = EntityKey::Post(Uuid::nil());
        let key2 = EntityKey::Post(Uuid::nil());
        assert_eq!(key1, key2);

        let key3 = EntityKey::PostSlug("hello".to_string());
        let key4 = EntityKey::PostSlug("hello".to_string());
        assert_eq!(key3, key4);

        assert_ne!(key1, EntityKey::Page(Uuid::nil()));
    }

    #[test]
    fn cache_key_hash_consistency() {
        let key1 = CacheKey::L0(L0Key::PostBySlug("test".to_string()));
        let key2 = CacheKey::L0(L0Key::PostBySlug("test".to_string()));

        let hash1 = hash_value(&key1);
        let hash2 = hash_value(&key2);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn l1_key_with_query_hash() {
        let key = L1Key::Response {
            format: OutputFormat::Html,
            path: "/posts/hello".to_string(),
            query_hash: hash_query("page=2"),
        };

        let cache_key = CacheKey::L1(key.clone());
        assert!(matches!(cache_key, CacheKey::L1(_)));

        // Same query produces same hash
        assert_eq!(hash_query("page=2"), hash_query("page=2"));
    }

    #[test]
    fn different_queries_produce_different_hashes() {
        let hash1 = hash_query("page=1");
        let hash2 = hash_query("page=2");
        assert_ne!(hash1, hash2);
    }
}
