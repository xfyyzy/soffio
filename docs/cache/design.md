# Soffio Cache System Design

> **Scope**: Single-site, single-process, no external dependencies (no Redis).  
> **Goal**: Provide code-level implementation guidance based on the [meta-design](./meta-design.md) and [candidates analysis](./candidates.md).

---

## 1. Module Structure

Create a new cache module at `src/cache/`:

```
src/cache/
├── mod.rs           # Module exports, global enable/disable based on config
├── config.rs        # Configuration struct from soffio.toml
├── keys.rs          # CacheKey and EntityKey enums
├── store.rs         # L0 storage (singleton/kv/lru) + L1 storage
├── registry.rs      # Bidirectional entity ↔ cache_key mapping
├── deps.rs          # DependencyCollector (task-local)
├── events.rs        # CacheEvent enum, in-memory queue
├── planner.rs       # Event → ConsumptionPlan (merge, dedupe)
├── consumer.rs      # Execute plan: invalidate + warm
└── middleware.rs    # L1 response cache middleware (axum layer)
```

---

## 2. Configuration (`soffio.toml`)

Cache is controlled via configuration (NOT feature flags). Extend the existing `[cache]` section in `soffio.toml.example`:

### 2.1 Configuration File Updates

```toml
[cache]
# Enable the L0 object/query cache.
# Env: SOFFIO__CACHE__ENABLE_L0_CACHE
# CLI: --cache-enable-l0-cache
enable_l0_cache = true

# Enable the L1 response cache for HTTP handlers.
# Env: SOFFIO__CACHE__ENABLE_L1_CACHE
# CLI: --cache-enable-l1-cache
enable_l1_cache = true

# Maximum cached posts in L0 KV cache.
# Env: SOFFIO__CACHE__L0_POST_LIMIT
# CLI: --cache-l0-post-limit
l0_post_limit = 500

# Maximum cached pages in L0 KV cache.
# Env: SOFFIO__CACHE__L0_PAGE_LIMIT
# CLI: --cache-l0-page-limit
l0_page_limit = 100

# Maximum cached API keys in L0 KV cache.
# Env: SOFFIO__CACHE__L0_API_KEY_LIMIT
# CLI: --cache-l0-api-key-limit
l0_api_key_limit = 100

# Maximum cached post list pages in L0 LRU cache.
# Env: SOFFIO__CACHE__L0_POST_LIST_LIMIT
# CLI: --cache-l0-post-list-limit
l0_post_list_limit = 50

# Maximum cached HTTP responses in L1 cache.
# Env: SOFFIO__CACHE__L1_RESPONSE_LIMIT
# CLI: --cache-l1-response-limit
l1_response_limit = 200

# Auto-consume interval in milliseconds for eventual consistency fallback.
# Env: SOFFIO__CACHE__AUTO_CONSUME_INTERVAL_MS
# CLI: --cache-auto-consume-interval-ms
auto_consume_interval_ms = 5000

# Maximum events per consumption batch.
# Env: SOFFIO__CACHE__CONSUME_BATCH_LIMIT
# CLI: --cache-consume-batch-limit
consume_batch_limit = 100
```

### 2.2 Rust Configuration Struct

```rust
// In src/cache/config.rs (mirroring existing config patterns)

use serde::Deserialize;

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
            l0_post_limit: 500,
            l0_page_limit: 100,
            l0_api_key_limit: 100,
            l0_post_list_limit: 50,
            l1_response_limit: 200,
            auto_consume_interval_ms: 5000,
            consume_batch_limit: 100,
        }
    }
}

impl CacheConfig {
    /// Returns true if any cache layer is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enable_l0_cache || self.enable_l1_cache
    }
}
```

### 2.3 Integration with Existing Config Infrastructure

Following the existing pattern in `src/config/`:

```rust
// Add to the main Config struct (src/config/mod.rs or equivalent)

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    // ... existing fields
    #[serde(default)]
    pub cache: CacheConfig,
}
```

---

## 3. Key Definitions (`keys.rs`)

### 3.1 EntityKey

Represents a domain entity or derived collection that can trigger cache invalidation.

```rust
use uuid::Uuid;

/// Identifies a domain entity or derived collection for cache invalidation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityKey {
    // Singletons
    SiteSettings,
    Navigation,

    // Content entities (by ID for write, slug for read)
    Post(Uuid),
    PostSlug(String),
    Page(Uuid),
    PageSlug(String),

    // Security
    ApiKey(String), // prefix

    // Derived collections (invalidated when any post/page changes)
    PostsIndex,         // Homepage, archives, tag/month filtered lists
    PostAggTags,        // Tag counts for sidebar
    PostAggMonths,      // Month counts for sidebar
    Feed,               // RSS/Atom
    Sitemap,
}
```

### 3.2 CacheKey

Represents a specific cache entry with all dimensions that affect output.

```rust
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
```

### 3.3 Hash Utilities

```rust
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

pub fn hash_filter(filter: &PostQueryFilter) -> u64 {
    let mut hasher = DefaultHasher::new();
    filter.tag.hash(&mut hasher);
    filter.month.hash(&mut hasher);
    filter.search.hash(&mut hasher);
    hasher.finish()
}

pub fn hash_cursor(cursor: Option<&PostCursor>) -> u64 {
    let mut hasher = DefaultHasher::new();
    cursor.hash(&mut hasher);
    hasher.finish()
}

pub fn hash_query(query: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    hasher.finish()
}
```

---

## 4. Storage Implementation (`store.rs`)

### 4.1 L0 Store

```rust
use std::sync::RwLock;
use std::collections::HashMap;
use lru::LruCache;
use std::num::NonZeroUsize;

use crate::domain::entities::{
    PostRecord, PageRecord, SiteSettingsRecord, NavigationItemRecord,
};
use crate::domain::api_keys::ApiKeyRecord;
use crate::domain::posts::{MonthCount, PostTagCount};

/// L0 object/query cache storage.
pub struct L0Store {
    config: CacheConfig,
    
    // Singletons (no eviction needed)
    site_settings: RwLock<Option<SiteSettingsRecord>>,
    navigation: RwLock<Option<Vec<NavigationItemRecord>>>,
    tag_counts: RwLock<Option<Vec<PostTagCount>>>,
    month_counts: RwLock<Option<Vec<MonthCount>>>,

    // KV caches (with LRU eviction)
    posts_by_id: RwLock<LruCache<Uuid, PostRecord>>,
    posts_by_slug: RwLock<LruCache<String, PostRecord>>,
    pages_by_id: RwLock<LruCache<Uuid, PageRecord>>,
    pages_by_slug: RwLock<LruCache<String, PageRecord>>,
    api_keys_by_prefix: RwLock<LruCache<String, ApiKeyRecord>>,

    // List cache (high cardinality, strict LRU)
    post_lists: RwLock<LruCache<(u64, u64), CursorPage<PostRecord>>>,
}

impl L0Store {
    pub fn new(config: &CacheConfig) -> Self {
        Self {
            config: config.clone(),
            site_settings: RwLock::new(None),
            navigation: RwLock::new(None),
            tag_counts: RwLock::new(None),
            month_counts: RwLock::new(None),
            posts_by_id: RwLock::new(LruCache::new(
                NonZeroUsize::new(config.l0_post_limit).unwrap()
            )),
            posts_by_slug: RwLock::new(LruCache::new(
                NonZeroUsize::new(config.l0_post_limit).unwrap()
            )),
            pages_by_id: RwLock::new(LruCache::new(
                NonZeroUsize::new(config.l0_page_limit).unwrap()
            )),
            pages_by_slug: RwLock::new(LruCache::new(
                NonZeroUsize::new(config.l0_page_limit).unwrap()
            )),
            api_keys_by_prefix: RwLock::new(LruCache::new(
                NonZeroUsize::new(config.l0_api_key_limit).unwrap()
            )),
            post_lists: RwLock::new(LruCache::new(
                NonZeroUsize::new(config.l0_post_list_limit).unwrap()
            )),
        }
    }

    // --- Singleton getters/setters ---

    pub fn get_site_settings(&self) -> Option<SiteSettingsRecord> {
        self.site_settings.read().unwrap().clone()
    }

    pub fn set_site_settings(&self, value: SiteSettingsRecord) {
        *self.site_settings.write().unwrap() = Some(value);
    }

    pub fn invalidate_site_settings(&self) {
        *self.site_settings.write().unwrap() = None;
    }

    // ... similar for navigation, tag_counts, month_counts

    // --- KV getters/setters ---

    pub fn get_post_by_slug(&self, slug: &str) -> Option<PostRecord> {
        self.posts_by_slug.write().unwrap().get(slug).cloned()
    }

    pub fn set_post(&self, post: PostRecord) {
        let mut by_id = self.posts_by_id.write().unwrap();
        let mut by_slug = self.posts_by_slug.write().unwrap();
        by_id.put(post.id, post.clone());
        by_slug.put(post.slug.clone(), post);
    }

    pub fn invalidate_post(&self, id: Uuid, slug: &str) {
        self.posts_by_id.write().unwrap().pop(&id);
        self.posts_by_slug.write().unwrap().pop(slug);
    }

    // ... similar for pages, api_keys

    // --- List cache ---

    pub fn get_post_list(&self, filter_hash: u64, cursor_hash: u64) 
        -> Option<CursorPage<PostRecord>> 
    {
        self.post_lists.write().unwrap()
            .get(&(filter_hash, cursor_hash))
            .cloned()
    }

    pub fn set_post_list(
        &self, 
        filter_hash: u64, 
        cursor_hash: u64, 
        page: CursorPage<PostRecord>
    ) {
        self.post_lists.write().unwrap()
            .put((filter_hash, cursor_hash), page);
    }

    pub fn invalidate_all_post_lists(&self) {
        self.post_lists.write().unwrap().clear();
    }
}
```

### 4.2 L1 Store

```rust
use bytes::Bytes;
use axum::http::HeaderMap;

/// Cached HTTP response.
#[derive(Clone)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Bytes,
}

/// L1 response cache storage.
pub struct L1Store {
    responses: RwLock<LruCache<L1Key, CachedResponse>>,
}

impl L1Store {
    pub fn new(config: &CacheConfig) -> Self {
        Self {
            responses: RwLock::new(LruCache::new(
                NonZeroUsize::new(config.l1_response_limit).unwrap()
            )),
        }
    }

    pub fn get(&self, key: &L1Key) -> Option<CachedResponse> {
        self.responses.write().unwrap().get(key).cloned()
    }

    pub fn set(&self, key: L1Key, response: CachedResponse) {
        self.responses.write().unwrap().put(key, response);
    }

    pub fn invalidate(&self, key: &L1Key) {
        self.responses.write().unwrap().pop(key);
    }

    pub fn invalidate_all(&self) {
        self.responses.write().unwrap().clear();
    }
}
```

---

## 5. Bidirectional Registry (`registry.rs`)

```rust
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

/// Tracks entity → cache_keys and cache_key → entities mappings.
pub struct CacheRegistry {
    entity_to_keys: RwLock<HashMap<EntityKey, HashSet<CacheKey>>>,
    key_to_entities: RwLock<HashMap<CacheKey, HashSet<EntityKey>>>,
}

impl CacheRegistry {
    pub fn new() -> Self {
        Self {
            entity_to_keys: RwLock::new(HashMap::new()),
            key_to_entities: RwLock::new(HashMap::new()),
        }
    }

    /// Register a cache entry with its dependent entities.
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
        self.entity_to_keys.read().unwrap()
            .get(entity)
            .cloned()
            .unwrap_or_default()
    }

    /// Remove a cache key and clean up entity mappings.
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
}
```

---

## 6. Dependency Collector (`deps.rs`)

Uses `tokio::task_local!` for zero-cost dependency tracking during request processing.

```rust
use std::cell::RefCell;
use std::collections::HashSet;

tokio::task_local! {
    static DEPS: RefCell<HashSet<EntityKey>>;
}

/// Initialize dependency collector for current task.
pub fn init_collector() {
    DEPS.scope(RefCell::new(HashSet::new()), async {});
}

/// Record an entity dependency (called from service layer).
pub fn record(entity: EntityKey) {
    let _ = DEPS.try_with(|deps| {
        deps.borrow_mut().insert(entity);
    });
}

/// Collect all recorded dependencies (called at request end).
pub fn collect() -> HashSet<EntityKey> {
    DEPS.try_with(|deps| deps.borrow().clone())
        .unwrap_or_default()
}

/// Run async block with dependency collector.
pub async fn with_collector<F, R>(f: F) -> (R, HashSet<EntityKey>)
where
    F: std::future::Future<Output = R>,
{
    let deps = RefCell::new(HashSet::new());
    let result = DEPS.scope(deps.clone(), f).await;
    (result, deps.into_inner())
}
```

### 6.1 Service Layer Instrumentation Points

Low-intrusion changes to `FeedService` and repository reads:

```rust
// In src/application/feed.rs

impl FeedService {
    pub fn load_site_settings(&self) -> Result<SiteSettingsRecord, FeedError> {
        // Record dependency before read
        crate::cache::deps::record(EntityKey::SiteSettings);
        
        tokio::runtime::Handle::current().block_on(async {
            self.settings.load_site_settings().await
        }).map_err(FeedError::from)
    }

    pub fn page_context(...) -> Result<PageContext, FeedError> {
        // Record derived collection dependencies
        crate::cache::deps::record(EntityKey::PostsIndex);
        crate::cache::deps::record(EntityKey::PostAggTags);
        crate::cache::deps::record(EntityKey::PostAggMonths);
        // ... existing logic
    }

    pub fn post_detail(&self, slug: &str) -> Result<Option<PostDetailContext>, FeedError> {
        crate::cache::deps::record(EntityKey::PostSlug(slug.to_string()));
        // ... existing logic
    }
}
```

---

## 7. Event System (`events.rs`)

### 7.1 Event Definitions

```rust
use time::OffsetDateTime;
use uuid::Uuid;

/// Monotonic epoch for ordering events.
pub type Epoch = u64;

/// Cache event with idempotency and ordering support.
#[derive(Debug, Clone)]
pub struct CacheEvent {
    pub id: Uuid,           // Idempotency key
    pub epoch: Epoch,       // For ordering
    pub kind: EventKind,
    pub timestamp: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub enum EventKind {
    // Singletons
    SiteSettingsUpdated,
    NavigationUpdated,

    // Content
    PostUpserted { post_id: Uuid, slug: String },
    PostDeleted { post_id: Uuid, slug: String },
    PageUpserted { page_id: Uuid, slug: String },
    PageDeleted { page_id: Uuid, slug: String },

    // Security
    ApiKeyUpserted { prefix: String },
    ApiKeyRevoked { prefix: String },

    // Startup
    WarmupOnStartup,
}

impl CacheEvent {
    pub fn new(kind: EventKind, epoch: Epoch) -> Self {
        Self {
            id: Uuid::now_v7(),
            epoch,
            kind,
            timestamp: OffsetDateTime::now_utc(),
        }
    }
}
```

### 7.2 Event Queue

```rust
use std::collections::VecDeque;
use std::sync::{Mutex, atomic::{AtomicU64, Ordering}};
use tracing::info;

pub struct EventQueue {
    queue: Mutex<VecDeque<CacheEvent>>,
    epoch_counter: AtomicU64,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            epoch_counter: AtomicU64::new(0),
        }
    }

    pub fn next_epoch(&self) -> Epoch {
        self.epoch_counter.fetch_add(1, Ordering::SeqCst)
    }

    pub fn publish(&self, kind: EventKind) {
        let epoch = self.next_epoch();
        let event = CacheEvent::new(kind.clone(), epoch);
        
        // Observable: log event enqueue
        info!(
            event_id = %event.id,
            event_epoch = event.epoch,
            event_kind = ?kind,
            "Cache event enqueued"
        );
        
        self.queue.lock().unwrap().push_back(event);
    }

    pub fn drain(&self, limit: usize) -> Vec<CacheEvent> {
        let mut queue = self.queue.lock().unwrap();
        let count = limit.min(queue.len());
        queue.drain(..count).collect()
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }
}
```

---

## 8. Consumption Planner (`planner.rs`)

### 8.1 Consumption Plan

```rust
use std::collections::HashSet;

/// Actions to execute for cache consistency.
#[derive(Debug, Default)]
pub struct ConsumptionPlan {
    // Entities to invalidate
    pub invalidate_entities: HashSet<EntityKey>,
    
    // Specific warm actions
    pub warm_site_settings: bool,
    pub warm_navigation: bool,
    pub warm_navigation_pages: bool,  // Pages linked from visible navigation
    pub warm_aggregations: bool,
    pub warm_posts: HashSet<Uuid>,
    pub warm_pages: HashSet<Uuid>,
    pub warm_homepage: bool,
    pub warm_feed: bool,
    pub warm_sitemap: bool,
}

impl std::fmt::Display for ConsumptionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConsumptionPlan {{ invalidate: {}, warm_settings: {}, warm_nav: {}, warm_nav_pages: {}, warm_agg: {}, warm_posts: {}, warm_pages: {}, warm_homepage: {}, warm_feed: {}, warm_sitemap: {} }}",
            self.invalidate_entities.len(),
            self.warm_site_settings,
            self.warm_navigation,
            self.warm_navigation_pages,
            self.warm_aggregations,
            self.warm_posts.len(),
            self.warm_pages.len(),
            self.warm_homepage,
            self.warm_feed,
            self.warm_sitemap,
        )
    }
}
```

### 8.2 Plan Generation

```rust
impl ConsumptionPlan {
    /// Merge multiple events into an optimized plan.
    pub fn from_events(events: Vec<CacheEvent>) -> Self {
        let mut plan = Self::default();
        let mut seen_ids = HashSet::new();
        
        // Dedupe by event ID
        let events: Vec<_> = events.into_iter()
            .filter(|e| seen_ids.insert(e.id))
            .collect();

        // Group by entity, keep latest epoch
        let mut post_epochs: HashMap<Uuid, (Epoch, EventKind)> = HashMap::new();
        let mut page_epochs: HashMap<Uuid, (Epoch, EventKind)> = HashMap::new();

        for event in events {
            match &event.kind {
                EventKind::SiteSettingsUpdated => {
                    plan.invalidate_entities.insert(EntityKey::SiteSettings);
                    plan.warm_site_settings = true;
                }
                EventKind::NavigationUpdated => {
                    plan.invalidate_entities.insert(EntityKey::Navigation);
                    plan.warm_navigation = true;
                    plan.warm_navigation_pages = true; // Warm pages linked from navigation
                }
                EventKind::PostUpserted { post_id, slug } |
                EventKind::PostDeleted { post_id, slug } => {
                    let entry = post_epochs.entry(*post_id);
                    entry.and_modify(|(e, k)| {
                        if event.epoch > *e {
                            *e = event.epoch;
                            *k = event.kind.clone();
                        }
                    }).or_insert((event.epoch, event.kind.clone()));
                }
                EventKind::PageUpserted { page_id, slug } |
                EventKind::PageDeleted { page_id, slug } => {
                    let entry = page_epochs.entry(*page_id);
                    entry.and_modify(|(e, k)| {
                        if event.epoch > *e {
                            *e = event.epoch;
                            *k = event.kind.clone();
                        }
                    }).or_insert((event.epoch, event.kind.clone()));
                }
                EventKind::ApiKeyUpserted { prefix } |
                EventKind::ApiKeyRevoked { prefix } => {
                    plan.invalidate_entities.insert(EntityKey::ApiKey(prefix.clone()));
                }
                EventKind::WarmupOnStartup => {
                    plan.warm_site_settings = true;
                    plan.warm_navigation = true;
                    plan.warm_navigation_pages = true; // Warm pages linked from visible navigation
                    plan.warm_aggregations = true;
                    plan.warm_homepage = true;
                    plan.warm_feed = true;
                    plan.warm_sitemap = true;
                }
            }
        }

        // Process post events
        let mut any_post_changed = false;
        for (post_id, (_, kind)) in post_epochs {
            any_post_changed = true;
            match kind {
                EventKind::PostDeleted { slug, .. } => {
                    plan.invalidate_entities.insert(EntityKey::Post(post_id));
                    plan.invalidate_entities.insert(EntityKey::PostSlug(slug));
                }
                EventKind::PostUpserted { slug, .. } => {
                    plan.invalidate_entities.insert(EntityKey::Post(post_id));
                    plan.invalidate_entities.insert(EntityKey::PostSlug(slug));
                    plan.warm_posts.insert(post_id);
                }
                _ => {}
            }
        }

        // If any post changed, invalidate derived collections
        if any_post_changed {
            plan.invalidate_entities.insert(EntityKey::PostsIndex);
            plan.invalidate_entities.insert(EntityKey::PostAggTags);
            plan.invalidate_entities.insert(EntityKey::PostAggMonths);
            plan.invalidate_entities.insert(EntityKey::Feed);
            plan.invalidate_entities.insert(EntityKey::Sitemap);
            plan.warm_aggregations = true;
            plan.warm_homepage = true;
            plan.warm_feed = true;
            plan.warm_sitemap = true;
        }

        // Process page events
        for (page_id, (_, kind)) in page_epochs {
            match kind {
                EventKind::PageDeleted { slug, .. } => {
                    plan.invalidate_entities.insert(EntityKey::Page(page_id));
                    plan.invalidate_entities.insert(EntityKey::PageSlug(slug));
                }
                EventKind::PageUpserted { slug, .. } => {
                    plan.invalidate_entities.insert(EntityKey::Page(page_id));
                    plan.invalidate_entities.insert(EntityKey::PageSlug(slug));
                    plan.warm_pages.insert(page_id);
                }
                _ => {}
            }
            plan.invalidate_entities.insert(EntityKey::Sitemap);
            plan.warm_sitemap = true;
        }

        plan
    }
}
```

---

## 9. Consumer (`consumer.rs`)

### 9.1 Consumer Service

```rust
use std::sync::Arc;
use tracing::{info, warn, instrument};

pub struct CacheConsumer {
    config: CacheConfig,
    l0: Arc<L0Store>,
    l1: Arc<L1Store>,
    registry: Arc<CacheRegistry>,
    queue: Arc<EventQueue>,
    repos: Arc<PostgresRepositories>,
}

impl CacheConsumer {
    /// Consume pending events and execute plan.
    /// Returns true if any events were processed.
    #[instrument(skip(self))]
    pub async fn consume(&self) -> bool {
        let events = self.queue.drain(self.config.consume_batch_limit);
        if events.is_empty() {
            return false;
        }

        let event_count = events.len();
        let event_ids: Vec<_> = events.iter().map(|e| e.id).collect();
        let plan = ConsumptionPlan::from_events(events);

        // Observable: log consumption start with plan details
        info!(
            event_count,
            event_ids = ?event_ids,
            plan = %plan,
            "Cache consumption starting"
        );

        // Phase 1: Invalidate L0
        self.invalidate_l0(&plan);

        // Phase 2: Invalidate L1 using registry
        self.invalidate_l1(&plan);

        // Phase 3: Warm (async, non-blocking)
        self.warm(&plan).await;

        // Observable: log consumption complete
        info!(
            event_count,
            invalidated = plan.invalidate_entities.len(),
            "Cache consumption complete"
        );

        true
    }

    fn invalidate_l0(&self, plan: &ConsumptionPlan) {
        for entity in &plan.invalidate_entities {
            match entity {
                EntityKey::SiteSettings => self.l0.invalidate_site_settings(),
                EntityKey::Navigation => self.l0.invalidate_navigation(),
                EntityKey::Post(id) => {
                    if let Some(post) = self.l0.get_post_by_id(id) {
                        self.l0.invalidate_post(*id, &post.slug);
                    }
                }
                EntityKey::PostSlug(slug) => {
                    if let Some(post) = self.l0.get_post_by_slug(slug) {
                        self.l0.invalidate_post(post.id, slug);
                    }
                }
                EntityKey::PostsIndex => self.l0.invalidate_all_post_lists(),
                EntityKey::PostAggTags => self.l0.invalidate_tag_counts(),
                EntityKey::PostAggMonths => self.l0.invalidate_month_counts(),
                // ... similar for pages, api_keys
                _ => {}
            }
        }
    }

    fn invalidate_l1(&self, plan: &ConsumptionPlan) {
        for entity in &plan.invalidate_entities {
            let keys = self.registry.keys_for_entity(entity);
            for key in keys {
                if let CacheKey::L1(l1_key) = &key {
                    self.l1.invalidate(l1_key);
                }
                self.registry.unregister(&key);
            }
        }
    }

    async fn warm(&self, plan: &ConsumptionPlan) {
        if plan.warm_site_settings {
            if let Ok(settings) = self.repos.load_site_settings().await {
                self.l0.set_site_settings(settings);
            }
        }

        if plan.warm_navigation {
            // Load visible navigation items
            if let Ok(nav) = self.repos.list_navigation(Some(true), ...).await {
                self.l0.set_navigation(nav.items.clone());
                
                // Warm pages linked from visible navigation
                if plan.warm_navigation_pages {
                    for item in &nav.items {
                        if let Some(page_id) = item.destination_page_id {
                            if let Ok(Some(page)) = self.repos.find_page_by_id(page_id).await {
                                self.l0.set_page(page);
                            }
                        }
                    }
                }
            }
        }

        if plan.warm_aggregations {
            if let Ok(tags) = self.repos.list_tag_counts(..).await {
                self.l0.set_tag_counts(tags);
            }
            if let Ok(months) = self.repos.list_month_counts(..).await {
                self.l0.set_month_counts(months);
            }
        }

        // Warm individual posts
        for post_id in &plan.warm_posts {
            if let Ok(Some(post)) = self.repos.find_by_id(*post_id).await {
                self.l0.set_post(post);
            }
        }

        // Warm individual pages
        for page_id in &plan.warm_pages {
            if let Ok(Some(page)) = self.repos.find_page_by_id(*page_id).await {
                self.l0.set_page(page);
            }
        }

        // ... similar for homepage list, feed, sitemap
    }
}
```

### 9.2 Trigger Points

Cache consumption is triggered from service layer, not middleware, ensuring all write sources (Admin UI, API, Jobs) follow the same path.

```rust
// In application/repos.rs or a new application/cache_trigger.rs

pub struct CacheTrigger {
    config: CacheConfig,
    queue: Arc<EventQueue>,
    consumer: Arc<CacheConsumer>,
}

impl CacheTrigger {
    /// Publish event and optionally consume immediately.
    pub async fn trigger(&self, kind: EventKind, consume_now: bool) {
        if !self.config.is_enabled() {
            return;
        }
        
        self.queue.publish(kind);
        
        if consume_now {
            self.consumer.consume().await;
        }
    }
}
```

---

## 10. L1 Response Cache Middleware (`middleware.rs`)

```rust
use axum::{
    body::Body,
    http::{Request, Response, StatusCode, Method},
    middleware::Next,
};
use bytes::Bytes;

/// Middleware for L1 response caching.
/// Only caches GET requests to public routes.
pub async fn response_cache_layer(
    State(cache): State<Arc<CacheState>>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    // Skip if L1 cache disabled
    if !cache.config.enable_l1_cache {
        return next.run(request).await;
    }

    // Only cache GET requests
    if request.method() != Method::GET {
        return next.run(request).await;
    }

    // Build cache key
    let path = request.uri().path().to_string();
    let query = request.uri().query().unwrap_or("");
    let format = detect_format(&request);
    
    let l1_key = L1Key::Response {
        format,
        path: path.clone(),
        query_hash: hash_query(query),
    };

    // Check cache
    if let Some(cached) = cache.l1.get(&l1_key) {
        return build_response(cached);
    }

    // Run with dependency collector
    let (response, deps) = deps::with_collector(next.run(request)).await;

    // Only cache successful responses
    if response.status() == StatusCode::OK {
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, 1024 * 1024).await
            .unwrap_or_default();
        
        let cached = CachedResponse {
            status: parts.status.as_u16(),
            headers: parts.headers.clone(),
            body: bytes.clone(),
        };

        cache.l1.set(l1_key.clone(), cached);
        cache.registry.register(CacheKey::L1(l1_key), deps);

        Response::from_parts(parts, Body::from(bytes))
    } else {
        response
    }
}

fn detect_format(request: &Request<Body>) -> OutputFormat {
    let path = request.uri().path();
    if path.ends_with("/rss.xml") || path.ends_with("/feed") {
        OutputFormat::Rss
    } else if path.ends_with("/atom.xml") {
        OutputFormat::Atom  
    } else if path.ends_with("/sitemap.xml") {
        OutputFormat::Sitemap
    } else if path.ends_with("/favicon.svg") {
        OutputFormat::Favicon
    } else if request.headers()
        .get("Accept")
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap_or("")
        .contains("application/json") 
    {
        OutputFormat::Json
    } else {
        OutputFormat::Html
    }
}
```

---

## 11. Write Source Matrix

All write operations must trigger cache events. Coverage:

| Write Source | Location | Events Published |
|--------------|----------|------------------|
| Admin UI: Update Settings | `src/infra/http/admin/settings/handlers.rs` | `SiteSettingsUpdated` |
| Admin UI: Update Navigation | `src/infra/http/admin/navigation/handlers.rs` | `NavigationUpdated` |
| Admin UI: Create/Update Post | `src/infra/http/admin/posts/crud.rs` | `PostUpserted` |
| Admin UI: Delete Post | `src/infra/http/admin/posts/crud.rs` | `PostDeleted` |
| Admin UI: Create/Update Page | `src/infra/http/admin/pages/handlers.rs` | `PageUpserted` |
| Admin UI: Delete Page | `src/infra/http/admin/pages/handlers.rs` | `PageDeleted` |
| Public API: CRUD Posts | `src/infra/http/api/handlers/posts.rs` | `PostUpserted/PostDeleted` |
| Public API: CRUD Pages | `src/infra/http/api/handlers/pages.rs` | `PageUpserted/PageDeleted` |
| Public API: CRUD Navigation | `src/infra/http/api/handlers/navigation.rs` | `NavigationUpdated` |
| Public API: Settings | `src/infra/http/api/handlers/settings.rs` | `SiteSettingsUpdated` |
| Jobs: Render Post | `src/application/jobs/` | `PostUpserted` (after render) |
| Jobs: Render Page | `src/application/jobs/` | `PageUpserted` (after render) |
| Snapshot Rollback | `src/application/snapshot_preview.rs` | `PostUpserted/PageUpserted` |

---

## 12. Startup Warmup

```rust
// In src/main.rs during server initialization

async fn warmup_cache(cache: Arc<CacheState>) {
    if !cache.config.is_enabled() {
        return;
    }
    
    cache.queue.publish(EventKind::WarmupOnStartup);
    cache.consumer.consume().await;
    
    info!("Cache warmup complete");
}
```

Priority warmup items:
1. `SiteSettings` - needed for every page
2. `Navigation` - needed for header
3. **Navigation-linked internal pages** - directly accessible from homepage
4. `TagCounts` / `MonthCounts` - sidebar aggregations
5. Homepage first page
6. RSS/Sitemap
7. Most recent N posts

---

## 13. Observability

### 13.1 Event Lifecycle Logging

All cache events are logged through tracing:

```rust
// Event enqueue (in events.rs)
info!(
    event_id = %event.id,
    event_epoch = event.epoch,
    event_kind = ?kind,
    queue_len = self.len(),
    "Cache event enqueued"
);

// Consumption start (in consumer.rs)
info!(
    event_count,
    event_ids = ?event_ids,
    plan = %plan,
    "Cache consumption starting"
);

// Consumption complete (in consumer.rs)
info!(
    event_count,
    invalidated = plan.invalidate_entities.len(),
    warmed_posts = plan.warm_posts.len(),
    warmed_pages = plan.warm_pages.len(),
    duration_ms,
    "Cache consumption complete"
);
```

### 13.2 Cache Hit/Miss Logging

```rust
// In store.rs
impl L0Store {
    pub fn get_post_by_slug(&self, slug: &str) -> Option<PostRecord> {
        let result = self.posts_by_slug.write().unwrap().get(slug).cloned();
        
        if result.is_some() {
            tracing::debug!(cache = "l0", entity = "post", slug, outcome = "hit");
        } else {
            tracing::debug!(cache = "l0", entity = "post", slug, outcome = "miss");
        }
        
        result
    }
}
```

### 13.3 Key Metrics

| Metric | Description | Log Field |
|--------|-------------|-----------|
| `cache_event_enqueued` | Event added to queue | `event_id`, `event_kind`, `queue_len` |
| `cache_consumption_started` | Batch consumption begins | `event_count`, `event_ids`, `plan` |
| `cache_consumption_complete` | Batch consumption ends | `event_count`, `invalidated`, `duration_ms` |
| `cache_l0_hit` | L0 cache hit | `entity`, `key`, `outcome=hit` |
| `cache_l0_miss` | L0 cache miss | `entity`, `key`, `outcome=miss` |
| `cache_l1_hit` | L1 response cache hit | `format`, `path`, `outcome=hit` |
| `cache_l1_miss` | L1 response cache miss | `format`, `path`, `outcome=miss` |

---

## 14. Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
lru = "0.16.2"
```

The `lru` crate provides a simple, well-tested LRU cache implementation.

---

## 15. Implementation Phases

### Phase 1: Core Infrastructure
1. Create `src/cache/` module structure
2. Implement `config.rs` with configuration struct
3. Update `soffio.toml.example` with new cache config options
4. Implement `keys.rs` with `EntityKey` and `CacheKey`
5. Implement `store.rs` with L0 storage
6. Implement `registry.rs` for bidirectional mapping
7. Add `lru` dependency

### Phase 2: Event System
1. Implement `events.rs` with event queue and observability logging
2. Implement `planner.rs` for event merging
3. Implement `consumer.rs` for plan execution with observability

### Phase 3: Integration
1. Add cache triggers to all write operations (Admin, API, Jobs)
2. Instrument `FeedService` with dependency recording
3. Implement startup warmup (including navigation-linked pages)

### Phase 4: L1 Response Cache
1. Implement `deps.rs` with task-local collector
2. Implement `middleware.rs` for response caching
3. Wire middleware into public router

### Phase 5: Testing & Verification
1. Implement cache consistency live tests
2. Integrate live tests into CI workflow
3. Add observability instrumentation to all components

---

## 16. Testing Strategy

### 16.1 Unit Tests

- `keys.rs`: Hash consistency for filters/cursors
- `registry.rs`: Bidirectional mapping correctness
- `planner.rs`: Event merging and deduplication
- `store.rs`: LRU eviction behavior

### 16.2 Live Tests

Following the existing `tests/live_api.rs` pattern, create `tests/live_cache.rs`:

```rust
//! Live cache consistency tests against a running soffio instance.
//!
//! - Tests cache invalidation and consistency after write operations.
//! - Marked `#[ignore]` so it only runs after seeding data and starting server.

use reqwest::{Client, StatusCode};
use serde_json::json;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
#[ignore]
async fn live_cache_consistency_post_update() -> TestResult<()> {
    let config = load_config()?;
    let client = Client::builder().build()?;
    let base = config.base_url.trim_end_matches('/').to_string();

    // 1. Create a post via API
    let (post_id, post_slug) = create_post(&client, &base, &config.keys.write).await?;
    
    // 2. Publish the post
    publish_post(&client, &base, &config.keys.write, &post_id).await?;
    
    // 3. Fetch public page (should see published content)
    let first_fetch = get_public_page(&client, &base, &post_slug).await?;
    assert!(first_fetch.contains("original content"));
    
    // 4. Update post via API
    update_post_body(&client, &base, &config.keys.write, &post_id, "updated content").await?;
    
    // 5. Immediately fetch public page again (cache should be invalidated)
    let second_fetch = get_public_page(&client, &base, &post_slug).await?;
    assert!(second_fetch.contains("updated content"), 
        "Cache inconsistency: public page still shows old content after update");
    
    // 6. Cleanup
    delete_post(&client, &base, &config.keys.write, &post_id).await?;
    
    Ok(())
}

#[tokio::test]
#[ignore]
async fn live_cache_consistency_navigation_update() -> TestResult<()> {
    // Test navigation cache invalidation
    // 1. Update navigation via API
    // 2. Verify homepage reflects new navigation immediately
}

#[tokio::test]
#[ignore]
async fn live_cache_consistency_settings_update() -> TestResult<()> {
    // Test site settings cache invalidation
    // 1. Update brand_title via API
    // 2. Verify homepage reflects new title immediately
}

#[tokio::test]
#[ignore]
async fn live_cache_consistency_aggregations() -> TestResult<()> {
    // Test aggregation cache invalidation
    // 1. Create a post with a new tag
    // 2. Publish the post
    // 3. Verify tag counts on homepage are updated
}
```

### 16.3 CI Integration

The live tests will be executed in CI following the existing pattern in `.github/workflows/ci.yml`:

```yaml
# After starting the server (existing pattern)
cargo test --target x86_64-unknown-linux-musl --test live_api -- --ignored --nocapture; \
cargo test --target x86_64-unknown-linux-musl --test live_cache -- --ignored --nocapture; \
```

Key cache consistency test scenarios:

| Test | Scenario | Verification |
|------|----------|--------------|
| `live_cache_consistency_post_update` | Update post body via API | Public page immediately shows new content |
| `live_cache_consistency_post_create` | Create and publish new post | Homepage immediately shows new post |
| `live_cache_consistency_post_delete` | Delete a published post | Homepage immediately removes post, 404 on detail |
| `live_cache_consistency_navigation_update` | Update navigation via API | Homepage header immediately reflects change |
| `live_cache_consistency_settings_update` | Update site settings | Homepage metadata immediately reflects change |
| `live_cache_consistency_aggregations` | Create post with new tag | Tag counts immediately updated on sidebar |
| `live_cache_consistency_feed` | Update post | RSS feed immediately shows updated content |
| `live_cache_consistency_sitemap` | Create/delete page | Sitemap immediately reflects change |

### 16.4 Test Configuration

Cache tests require the server to run with caching enabled. Update `tests/api_keys.seed.toml` or create `tests/cache.seed.toml`:

```toml
base_url = "http://127.0.0.1:3000"

[keys]
all = "sof_all_XXXXXXXX"
write = "sof_write_XXXXXXXX"
read = "sof_read_XXXXXXXX"
```

---

## Appendix A: Entity-Dependency Mapping

| Service Method | EntityKey Dependencies |
|----------------|------------------------|
| `FeedService::load_site_settings` | `SiteSettings` |
| `FeedService::page_context` | `PostsIndex`, `PostAggTags`, `PostAggMonths`, `SiteSettings`, `Navigation` |
| `FeedService::post_detail` | `PostSlug(slug)`, `SiteSettings` |
| `build_sitemap_xml` | `Sitemap`, all `PostSlug`, all `PageSlug` |
| `build_rss_xml` | `Feed`, `PostsIndex`, `SiteSettings` |
| `ApiMiddleware::validate_key` | `ApiKey(prefix)` |

---

## Appendix B: Event → Invalidation Mapping

| Event | Invalidated EntityKeys | Warm Actions |
|-------|------------------------|--------------|
| `SiteSettingsUpdated` | `SiteSettings` | Site settings |
| `NavigationUpdated` | `Navigation` | Visible navigation + linked internal pages |
| `PostUpserted` | `Post(id)`, `PostSlug(slug)`, `PostsIndex`, `PostAggTags`, `PostAggMonths`, `Feed`, `Sitemap` | Post, aggregations, homepage, feed, sitemap |
| `PostDeleted` | Same as above | Aggregations, homepage, feed, sitemap |
| `PageUpserted` | `Page(id)`, `PageSlug(slug)`, `Sitemap` | Page, sitemap |
| `PageDeleted` | Same as above | Sitemap |
| `ApiKeyUpserted` | `ApiKey(prefix)` | None |
| `ApiKeyRevoked` | `ApiKey(prefix)` | None |
| `WarmupOnStartup` | None | All singletons, navigation-linked pages, aggregations, homepage, feed, sitemap |
