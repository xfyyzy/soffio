//! Soffio Cache System
//!
//! Provides two-layer caching for the Soffio blog engine:
//!
//! - **L0 (Object Cache)**: Caches domain entities and query results
//! - **L1 (Response Cache)**: Caches rendered HTTP responses
//!
//! ## Configuration
//!
//! Cache behavior is controlled via `soffio.toml`:
//!
//! ```toml
//! [cache]
//! enable_l0_cache = true
//! enable_l1_cache = true
//! l0_post_limit = 500
//! l0_page_limit = 100
//! # ... see config.rs for all options
//! ```

mod config;
mod consumer;
pub mod deps;
mod events;
mod keys;
mod lock;
mod middleware;
mod planner;
mod registry;
mod store;
mod trigger;

pub use config::CacheConfig;
pub use consumer::CacheConsumer;
pub use events::{CacheEvent, Epoch, EventKind, EventQueue};
pub use keys::{
    CacheKey, EntityKey, L0Key, L1Key, OutputFormat, hash_cursor_str, hash_post_list_key,
    hash_query, hash_value,
};
pub use middleware::{CacheState, response_cache_layer};
pub use planner::ConsumptionPlan;
pub use registry::CacheRegistry;
pub use store::{CachedResponse, L0Store, L1Store};
pub use trigger::CacheTrigger;
