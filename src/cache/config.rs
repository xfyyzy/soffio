//! Cache configuration.
//!
//! Controls L0 object/query cache and L1 response cache via `soffio.toml`.

use std::num::NonZeroUsize;

use serde::Deserialize;

// Default values for cache configuration
const DEFAULT_L0_POST_LIMIT: usize = 500;
const DEFAULT_L0_PAGE_LIMIT: usize = 100;
const DEFAULT_L0_API_KEY_LIMIT: usize = 100;
const DEFAULT_L0_POST_LIST_LIMIT: usize = 50;
const DEFAULT_L1_RESPONSE_LIMIT: usize = 200;
const DEFAULT_AUTO_CONSUME_INTERVAL_MS: u64 = 5000;
const DEFAULT_CONSUME_BATCH_LIMIT: usize = 100;

/// Cache configuration from `soffio.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    /// Enable L0 object/query cache.
    pub enable_l0_cache: bool,
    /// Enable L1 response cache.
    pub enable_l1_cache: bool,
    /// Maximum posts in L0 KV cache.
    pub l0_post_limit: usize,
    /// Maximum pages in L0 KV cache.
    pub l0_page_limit: usize,
    /// Maximum API keys in L0 KV cache.
    pub l0_api_key_limit: usize,
    /// Maximum post list pages in L0 LRU cache.
    pub l0_post_list_limit: usize,
    /// Maximum HTTP responses in L1 cache.
    pub l1_response_limit: usize,
    /// Auto-consume interval (ms) for eventual consistency.
    pub auto_consume_interval_ms: u64,
    /// Maximum events per consumption batch.
    pub consume_batch_limit: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_l0_cache: true,
            enable_l1_cache: true,
            l0_post_limit: DEFAULT_L0_POST_LIMIT,
            l0_page_limit: DEFAULT_L0_PAGE_LIMIT,
            l0_api_key_limit: DEFAULT_L0_API_KEY_LIMIT,
            l0_post_list_limit: DEFAULT_L0_POST_LIST_LIMIT,
            l1_response_limit: DEFAULT_L1_RESPONSE_LIMIT,
            auto_consume_interval_ms: DEFAULT_AUTO_CONSUME_INTERVAL_MS,
            consume_batch_limit: DEFAULT_CONSUME_BATCH_LIMIT,
        }
    }
}

impl From<&crate::config::CacheSettings> for CacheConfig {
    fn from(settings: &crate::config::CacheSettings) -> Self {
        Self {
            enable_l0_cache: settings.enable_l0_cache,
            enable_l1_cache: settings.enable_l1_cache,
            l0_post_limit: settings.l0_post_limit,
            l0_page_limit: settings.l0_page_limit,
            l0_api_key_limit: settings.l0_api_key_limit,
            l0_post_list_limit: settings.l0_post_list_limit,
            l1_response_limit: settings.l1_response_limit,
            auto_consume_interval_ms: settings.auto_consume_interval_ms,
            consume_batch_limit: settings.consume_batch_limit,
        }
    }
}

impl CacheConfig {
    /// Returns true if any cache layer is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enable_l0_cache || self.enable_l1_cache
    }

    /// Returns the L0 post limit as NonZeroUsize, clamping to 1 if zero.
    pub fn l0_post_limit_non_zero(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.l0_post_limit).unwrap_or(NonZeroUsize::MIN)
    }

    /// Returns the L0 page limit as NonZeroUsize, clamping to 1 if zero.
    pub fn l0_page_limit_non_zero(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.l0_page_limit).unwrap_or(NonZeroUsize::MIN)
    }

    /// Returns the L0 API key limit as NonZeroUsize, clamping to 1 if zero.
    pub fn l0_api_key_limit_non_zero(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.l0_api_key_limit).unwrap_or(NonZeroUsize::MIN)
    }

    /// Returns the L0 post list limit as NonZeroUsize, clamping to 1 if zero.
    pub fn l0_post_list_limit_non_zero(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.l0_post_list_limit).unwrap_or(NonZeroUsize::MIN)
    }

    /// Returns the L1 response limit as NonZeroUsize, clamping to 1 if zero.
    pub fn l1_response_limit_non_zero(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.l1_response_limit).unwrap_or(NonZeroUsize::MIN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        let config = CacheConfig::default();
        assert!(config.enable_l0_cache);
        assert!(config.enable_l1_cache);
        assert_eq!(config.l0_post_limit, 500);
        assert_eq!(config.l0_page_limit, 100);
        assert_eq!(config.l0_api_key_limit, 100);
        assert_eq!(config.l0_post_list_limit, 50);
        assert_eq!(config.l1_response_limit, 200);
        assert_eq!(config.auto_consume_interval_ms, 5000);
        assert_eq!(config.consume_batch_limit, 100);
    }

    #[test]
    fn is_enabled_when_l0_only() {
        let config = CacheConfig {
            enable_l0_cache: true,
            enable_l1_cache: false,
            ..Default::default()
        };
        assert!(config.is_enabled());
    }

    #[test]
    fn is_enabled_when_l1_only() {
        let config = CacheConfig {
            enable_l0_cache: false,
            enable_l1_cache: true,
            ..Default::default()
        };
        assert!(config.is_enabled());
    }

    #[test]
    fn is_disabled_when_both_off() {
        let config = CacheConfig {
            enable_l0_cache: false,
            enable_l1_cache: false,
            ..Default::default()
        };
        assert!(!config.is_enabled());
    }

    #[test]
    fn non_zero_clamps_to_min() {
        let config = CacheConfig {
            l0_post_limit: 0,
            ..Default::default()
        };
        assert_eq!(config.l0_post_limit_non_zero().get(), 1);
    }
}
