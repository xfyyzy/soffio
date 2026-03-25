use metrics::counter;

use super::{
    CacheConfig, CachedResponse, L1Key, L1Store, METRIC_L1_EVICT_TOTAL, SOURCE, rw_read, rw_write,
};

impl L1Store {
    /// Create a new L1 store with the given configuration.
    pub fn new(config: &CacheConfig) -> Self {
        Self {
            responses: std::sync::RwLock::new(lru::LruCache::new(
                config.l1_response_limit_non_zero(),
            )),
        }
    }

    pub fn get(&self, key: &L1Key) -> Option<CachedResponse> {
        rw_write(&self.responses, SOURCE, "l1_get")
            .get(key)
            .cloned()
    }

    pub fn set(&self, key: L1Key, response: CachedResponse) -> Option<L1Key> {
        let evicted = rw_write(&self.responses, SOURCE, "l1_set").push(key, response);
        if evicted.is_some() {
            counter!(METRIC_L1_EVICT_TOTAL).increment(1);
        }
        evicted.map(|(evicted_key, _)| evicted_key)
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
