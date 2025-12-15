//! Bidirectional cache registry.
//!
//! Tracks the relationship between domain entities and cache entries,
//! enabling efficient invalidation when entities change.

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use super::keys::{CacheKey, EntityKey};

/// Tracks entity → cache_keys and cache_key → entities mappings.
///
/// This bidirectional mapping enables:
/// - Finding all cache entries affected by an entity change
/// - Cleaning up entity mappings when cache entries are evicted
pub struct CacheRegistry {
    /// Maps entities to all cache keys that depend on them
    entity_to_keys: RwLock<HashMap<EntityKey, HashSet<CacheKey>>>,
    /// Maps cache keys to all entities they depend on
    key_to_entities: RwLock<HashMap<CacheKey, HashSet<EntityKey>>>,
}

impl CacheRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            entity_to_keys: RwLock::new(HashMap::new()),
            key_to_entities: RwLock::new(HashMap::new()),
        }
    }

    /// Register a cache entry with its dependent entities.
    ///
    /// This creates bidirectional mappings so that:
    /// - When any entity changes, we can find all affected cache keys
    /// - When a cache entry is evicted, we can clean up entity mappings
    pub fn register(&self, cache_key: CacheKey, entities: HashSet<EntityKey>) {
        let mut e2k = self.entity_to_keys.write().unwrap();
        let mut k2e = self.key_to_entities.write().unwrap();

        for entity in &entities {
            e2k.entry(entity.clone())
                .or_default()
                .insert(cache_key.clone());
        }
        k2e.insert(cache_key, entities);
    }

    /// Get all cache keys affected by an entity change.
    pub fn keys_for_entity(&self, entity: &EntityKey) -> HashSet<CacheKey> {
        self.entity_to_keys
            .read()
            .unwrap()
            .get(entity)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all entities that a cache key depends on.
    pub fn entities_for_key(&self, cache_key: &CacheKey) -> HashSet<EntityKey> {
        self.key_to_entities
            .read()
            .unwrap()
            .get(cache_key)
            .cloned()
            .unwrap_or_default()
    }

    /// Remove a cache key and clean up entity mappings.
    ///
    /// Called when a cache entry is evicted or invalidated.
    pub fn unregister(&self, cache_key: &CacheKey) {
        let mut e2k = self.entity_to_keys.write().unwrap();
        let mut k2e = self.key_to_entities.write().unwrap();

        if let Some(entities) = k2e.remove(cache_key) {
            for entity in entities {
                if let Some(keys) = e2k.get_mut(&entity) {
                    keys.remove(cache_key);
                    if keys.is_empty() {
                        e2k.remove(&entity);
                    }
                }
            }
        }
    }

    /// Remove all mappings for an entity.
    ///
    /// Returns the set of cache keys that were affected.
    pub fn unregister_entity(&self, entity: &EntityKey) -> HashSet<CacheKey> {
        let mut e2k = self.entity_to_keys.write().unwrap();
        let mut k2e = self.key_to_entities.write().unwrap();

        let affected_keys = e2k.remove(entity).unwrap_or_default();

        for cache_key in &affected_keys {
            if let Some(entities) = k2e.get_mut(cache_key) {
                entities.remove(entity);
                // Note: We don't remove empty k2e entries here because the cache
                // entry may still be valid with other dependencies
            }
        }

        affected_keys
    }

    /// Clear all mappings.
    pub fn clear(&self) {
        self.entity_to_keys.write().unwrap().clear();
        self.key_to_entities.write().unwrap().clear();
    }

    /// Get the number of tracked entities.
    pub fn entity_count(&self) -> usize {
        self.entity_to_keys.read().unwrap().len()
    }

    /// Get the number of tracked cache keys.
    pub fn key_count(&self) -> usize {
        self.key_to_entities.read().unwrap().len()
    }
}

impl Default for CacheRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::cache::keys::L0Key;

    #[test]
    fn register_and_lookup() {
        let registry = CacheRegistry::new();

        let post_id = Uuid::new_v4();
        let entity = EntityKey::Post(post_id);
        let cache_key = CacheKey::L0(L0Key::PostById(post_id));

        let mut entities = HashSet::new();
        entities.insert(entity.clone());

        registry.register(cache_key.clone(), entities);

        // Can find cache key from entity
        let keys = registry.keys_for_entity(&entity);
        assert!(keys.contains(&cache_key));

        // Can find entity from cache key
        let found_entities = registry.entities_for_key(&cache_key);
        assert!(found_entities.contains(&entity));
    }

    #[test]
    fn unregister_cleans_up_mappings() {
        let registry = CacheRegistry::new();

        let post_id = Uuid::new_v4();
        let entity = EntityKey::Post(post_id);
        let cache_key = CacheKey::L0(L0Key::PostById(post_id));

        let mut entities = HashSet::new();
        entities.insert(entity.clone());

        registry.register(cache_key.clone(), entities);
        assert_eq!(registry.key_count(), 1);
        assert_eq!(registry.entity_count(), 1);

        registry.unregister(&cache_key);
        assert_eq!(registry.key_count(), 0);
        assert_eq!(registry.entity_count(), 0);
    }

    #[test]
    fn multiple_keys_for_same_entity() {
        let registry = CacheRegistry::new();

        let entity = EntityKey::SiteSettings;
        let key1 = CacheKey::L0(L0Key::SiteSettings);
        let key2 = CacheKey::L0(L0Key::Navigation); // Also depends on settings

        let mut entities = HashSet::new();
        entities.insert(entity.clone());

        registry.register(key1.clone(), entities.clone());
        registry.register(key2.clone(), entities);

        let keys = registry.keys_for_entity(&entity);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&key1));
        assert!(keys.contains(&key2));
    }

    #[test]
    fn unregister_entity_returns_affected_keys() {
        let registry = CacheRegistry::new();

        let entity = EntityKey::PostsIndex;
        let key1 = CacheKey::L0(L0Key::PostList {
            filter_hash: 0,
            cursor_hash: 0,
        });
        let key2 = CacheKey::L0(L0Key::PostList {
            filter_hash: 1,
            cursor_hash: 0,
        });

        let mut entities = HashSet::new();
        entities.insert(entity.clone());

        registry.register(key1.clone(), entities.clone());
        registry.register(key2.clone(), entities);

        let affected = registry.unregister_entity(&entity);
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&key1));
        assert!(affected.contains(&key2));
    }

    #[test]
    fn clear_removes_all_mappings() {
        let registry = CacheRegistry::new();

        let entity = EntityKey::SiteSettings;
        let cache_key = CacheKey::L0(L0Key::SiteSettings);

        let mut entities = HashSet::new();
        entities.insert(entity);

        registry.register(cache_key, entities);
        assert!(registry.key_count() > 0);

        registry.clear();
        assert_eq!(registry.key_count(), 0);
        assert_eq!(registry.entity_count(), 0);
    }
}
