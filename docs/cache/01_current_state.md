# Cache System — Current State (Dec 2025)

## What exists today
- **Response cache** (`src/infra/cache.rs`): in-memory `HashMap` keyed by request path+query. Stores full HTTP responses; no TTL/size cap. SEO strings stored separately (`SeoKey` map). Global `epoch` increments on `invalidate_all()`.
- **Cache warm debouncer** (`CacheWarmDebouncer`): 5s default window. Used to throttle warm jobs; shared across HTTP & workers.
- **Invalidation + warm helper** (`invalidate_and_enqueue_warm` in `src/application/jobs/cache.rs`):
  - Calls `cache.invalidate_all()` synchronously, then (if debounced) enqueues a `WarmCache` job carrying the cache epoch.
  - Epoch guard: warm job skips if cache epoch has advanced.
- **WarmCache job** (`JobType::WarmCache`, worker wired in `main.rs`): executes `CacheWarmer::warm_initial()` to rebuild hot paths (home, pinned tags, published pages, each post detail). Uses HashSets to dedupe warmed paths/posts.
- **HTTP cache usage**:
  - Public router wraps cacheable routes with `cache_public_responses` middleware (GET only; bypass on `X-Datastar-Request`). Key = `path_and_query`.
  - SEO endpoints (`/sitemap.xml`, `/rss.xml`, `/atom.xml`, `/robots.txt`) read/write `SeoKey` entries explicitly, warming on miss.
- **Write-triggered invalidation** (all fire-and-forget via `tokio::spawn`):
  - API router layer `invalidate_and_warm_cache` after successful non-GET responses (`src/infra/http/api/mod.rs`, middleware).
  - Admin router layer `invalidate_admin_writes` for admin writes.
  - Render jobs (`process_render_post_job`) for published posts after persisting render results.
  - Publish jobs (`process_publish_post_job`, `process_publish_page_job`).
- **Startup warm**: `warm_initial_cache()` called during startup; failure bubbles as `AppError`.
- **Config switches**: `cache.enable_response_cache` and `cache.enable_html_fragment_cache` exist in config/CLI but are not currently consulted in HTTP wiring (cache always active when code is built).
- **Observability**: structured logs for cache miss/hit/warm start/skip/fail; no metrics, no queue length visibility.
- **Tests**: `tests/cache_consistency.rs` documents current invalidation semantics; `tests/live_api.rs` E2E assumes immediate invalidation then warm. Warm job counts surfaced in admin/jobs endpoint and CSS badge (`static/admin/app/20-components.css`).
- **Schema/history**: migration `20251210120000_remove_invalidate_cache_jobs.up.sql` removed legacy invalidate job type (now synchronous helper). CHANGELOG entries highlight synchronous invalidation + debounced warm behavior.

## Gaps vs proposed “evented prewarm only” model
- Global invalidation clears all entries; precision is coarse and write-heavy paths pay cost even for small changes.
- Warm pipeline is per-invalidation and job-based; no event queue, no batch/merge across operations, no notion of “consume whole queue”.
- No auto-trigger when cache stays dirty for a while; relies solely on write-triggered invalidation.
- Debouncer prevents storms but also drops context (reason, affected keys) — cannot compute minimal warm set.
- Cache durability: in-memory only; restart loses both cache and pending warm intents (acceptable for new plan but must be explicit).
- Config toggles unused; cache cannot be disabled at runtime despite flags.
- Observability lacks counters for enqueue/flush/dedupe, queue age, or staleness window.

## Why the new plan improves things
- **Precision**: event->key mapping allows warming only the affected paths instead of blanket invalidation.
- **Batching**: consume entire event queue to dedupe and merge, reducing redundant renders/fetches during bursts.
- **Controlled staleness**: auto-trigger based on idle window caps worst-case cache age without synchronous invalidation.
- **Auditability**: structured logs/metrics on enqueue/flush/plan provide traceable cache behavior instead of opaque debounced drops.
- **Separation**: write paths emit cache events only; sync happens in a dedicated consumer, reducing business-path latency and coupling.
