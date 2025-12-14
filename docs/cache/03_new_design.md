# New Cache Architecture — Evented Batched Prewarm

## Goals
- No synchronous invalidation; rely on precise prewarm to converge.
- Event queue decoupled from business writes; consumer batches, dedupes, and executes minimal warm/evict plan.
- Bounded staleness via auto-trigger when queue stays non-empty past a configurable idle window (default 60s, min 30s, max 300s).
- In-memory only (per requirement); deterministic, auditable logs/metrics.
- Minimal intrusion to business logic: replace existing `invalidate_and_enqueue_warm` calls with lightweight event emits at the same boundaries.

## Scope & surfaces
- Public HTML cache (paths via `cache_public_responses` middleware).
- SEO fragments cache (`SeoKey` entries: sitemap, rss, atom, robots).
- Out of scope unless stated: mermaid cache, upload serving, HTML fragment cache flag (unchanged).

## Event model (precision by attribute)
- `CacheEvent { kind, at: Instant, reason: String, correlation_id?: Uuid, attrs: CacheAttrs }`
- `CacheAttrs` is enum-like but allows carrying fine-grained diffs so we can target minimal keys. Proposed kinds (derive `DedupKey`):
  - `PostChanged { slug, status_before?, status_after, tags_before?, tags_after?, month_before?, month_after?, title_changed: bool, body_changed: bool, summary_changed: bool }`
    - Keys: detail page only if published; homepage/tag/month pages and `/ui/posts` when visibility or tag set changes; SEO feeds only when published status/title/excerpt/body change (tag-only changes do not affect RSS/Atom content).
  - `PageChanged { slug, status_before?, status_after, nav_visible_before?, nav_visible_after?, title_changed: bool, body_changed: bool }`
    - Keys: page URL if published/visible; sitemap if published/title changed; nav-visible pages if nav visibility toggled.
  - `NavigationChanged { nav_items_hash_before?, nav_items_hash_after }`
    - Keys: homepage + all nav-visible pages. (Sitemap content is independent of navigation.)
  - `TagChanged { slug, pinned_before?, pinned_after?, name_changed: bool, description_changed: bool }`
    - Keys (accuracy-first): always `/tags/{slug}`; homepage/tag/month pages and `/ui/posts` that render posts containing this tag when `show_tag_aggregations` is enabled **and** either (a) the tag is pinned, or (b) the tag is within the displayed slice after `order_tags_with_pins` using `SiteSettingsRecord.tag_filter_limit` (pinned first, then count desc, then name). Tag name change does **not** affect RSS/Atom (feeds omit tags) but does affect any page where badges render the name.
  - `ChromeChanged { site_title_changed: bool, description_changed: bool, canonical_changed: bool }`
    - Keys: broad warm of homepage + published pages + SEO feeds when canonical/title shift.
  - `SettingsChanged { homepage_size_changed: bool, tag_filter_limit_changed: bool, show_tag_aggs_changed: bool, month_filter_limit_changed: bool, show_month_aggs_changed: bool, timezone_changed: bool, brand_changed: bool, footer_changed: bool, favicon_changed: bool }`
    - Keys: homepage + `/ui/posts` (page size, tag/month blocks), affected tag/month lists, all pages if timezone/brand/footer/favicon/public_site_url change; RSS/Atom if titles/descriptions/timezone/public_site_url change.
  - `SeoRefresh { scope: All | Sitemap | Rss | Atom | Robots }`
  - `PurgePath { path }` (for delete/unpublish to evict specific keys immediately in plan).
- Queue: `VecDeque<CacheEvent>` protected by `Mutex` + `last_enqueue` timestamp (for idle window watchdog).
- Dedup/merge: plan builder collapses events by `DedupKey` (slug/path/tag) keeping the latest payload; merges `TagChanged` into `PostChanged` when tag sets intersect; when conflicting status flips exist, keep the newest event and record `status_before`/`status_after` to drive eviction vs warm.

## Triggers (event emission)
- Replace every `invalidate_and_enqueue_warm` call with `cache_events.enqueue(...)`:
  - API + Admin write middleware (non-GET successful responses).
  - Render jobs after persisting rendered content for published posts/pages.
  - Publish jobs (scheduled publish completion).
- Additional emits to cover non-render writes that affect public cache but were previously implicit via invalidation:
  - Settings/chrome updates (`AdminSettingsService`).
  - Navigation updates (`AdminNavigationService`).
  - Tag pin/rename (`AdminTagService`).
  - Snapshot rollback / snapshot publish if they change rendered content.

## Sync trigger logic
- Manual trigger: specific business operations can call `cache_events.trigger_flush("reason")` to request immediate consume (e.g., publish, render completion, settings change).
- Idle watchdog: background task checks `queue_not_empty && oldest_event_age >= idle_window` → trigger flush. Idle window configurable (default 60s, bounds 30–300s).
- Backpressure: cap queue length (config, e.g., 1024). If exceeded, log and coalesce to a `FullRebuild` marker to avoid unbounded growth.

## Plan builder (consume whole queue)
1. Drain queue atomically (`Vec<CacheEvent>`).
2. Build `CacheSyncPlan`:
   - `paths_to_warm: HashSet<String>` (public cache keys)
   - `seo_to_warm: HashSet<SeoKey>`
   - `paths_to_evict: HashSet<String>` (for deletes/unpublish)
   - `full_rebuild: bool` fallback if plan is too large or queue overflowed.
3. Derive actions per event (precision):
   - `PostChanged`: `/posts/{slug}` if published; homepage `/` and `/ui/posts`; month archive `/months/{YYYY-MM}` if month exists **and** `show_month_aggregations`; tag pages for each tag; homepage tag list when `show_tag_aggregations` (tag counts change); sitemap/rss/atom when published status/title/excerpt/body changes.
   - `PageChanged`: `/{slug}` if published; navigation-dependent pages (same set as `CacheWarmer::warm_published_pages`) plus sitemap.
   - `NavigationChanged`: warm homepage and all nav-visible pages (`warm_published_pages`) because chrome includes nav links.
   - `TagChanged`: `/tags/{slug}`; homepage + `/ui/posts` + tag/month aggregates showing posts with this tag when `show_tag_aggregations` and tag is pinned or in visible slice; sitemap/rss/atom unaffected by name change.
   - `ChromeChanged` / `SettingsChanged`: broad warm when meta/canonical/timezone/brand/footer/favicon/public_site_url change; when `homepage_size`, `tag_filter_limit`, `show_tag_aggregations`, `month_filter_limit`, or `show_month_aggregations` change, warm homepage + `/ui/posts` + affected tag/month lists; timezone affects all date-rendering pages and feeds.
   - `PurgePath`: add to `paths_to_evict`.
   - `SeoRefresh`: add selected `SeoKey` entries.
4. Apply dedupe: HashSets ensure minimal path list; prefer most recent status (published vs draft) when conflicts.
5. Output plan with correlation id and stats (input events, dedupe counts, resulting path counts).

## Execution engine
- Introduce `CacheSyncService` with two collaborators:
  - `CacheEventQueue`: enqueue + flush trigger + watchdog.
  - `CacheWarmExecutor`: executes `CacheSyncPlan`.
- Executor actions order:
  1) Evict: implement `ResponseCache::evict(&str)` and `evict_seo(SeoKey)` for deletes/unpublish.
  2) Warm SEO: re-render sitemap/rss/atom/robots using existing HTTP handlers or dedicated functions.
  3) Warm paths: reuse/refactor `CacheWarmer` to accept a list of paths; internally use existing `FeedService`/`PageService` to render responses and `ResponseCache::store_response`.
- Concurrency: warm paths sequentially or with small bounded concurrency (config, default 4) to avoid DB pressure.
- Idempotency: warming overwrites existing entries; no epoch needed. Consumer drains all events each flush to guarantee monotonic convergence.

## Developer ergonomics (keep implementation low-friction)
- Provide helper modules alongside planner:
  - `TagSliceCalculator`: computes visible tags using `show_tag_aggregations`, `tag_filter_limit`, and `order_tags_with_pins` (pinned desc, count desc, name asc, slug asc) fed by `tags.list_with_counts()`.
  - `MonthSliceCalculator`: computes visible months using `show_month_aggregations` + `month_filter_limit` fed by `posts.list_month_counts()`.
  - `RouteBuilder`: canonicalizes cache keys for home, tag, month, post, page, `/ui/posts`, and SEO endpoints.
- Emit helpers per domain to avoid hand-crafting events:
  - `emit_post_changed(before?, after)` derives status/title/body/summary changes, month_before/after, tag sets, pinned flag.
  - `emit_page_changed(before?, after)` for published-state and nav visibility impacts.
  - `emit_tag_changed(before?, after)` for name/description/pin deltas.
  - `emit_navigation_changed(before, after)` using nav hash.
  - `emit_settings_changed(diff)` covering size/aggregation/timezone/meta/brand/footer/favicon/public_site_url.
- Flush safety: single-flight flush, queue overflow sets `full_rebuild`, config read from one source (`Settings`), and bounded warm concurrency.
- Failure policy: warming errors should log + continue; do not re-enqueue by default (best-effort). Document this in ops runbook.


## Consistency & SLA
- Staleness bound = min(manual triggers, idle_window). With default 60s idle window, worst-case stale window ≤ 60s under sustained write inactivity.
- Deletes/unpublish rely on explicit `PurgePath` events to remove stale entries; mandatory emission at delete/unpublish call sites.
- Startup: optionally run `CacheSyncService::flush_full("startup")` to warm baseline without global invalidation.

## Observability
- Logs (structured): enqueue `{event_kind, slug?, reason, queue_len}`, trigger `{reason, queued, oldest_age_ms}`, plan `{paths, seo, evicts, deduped_from}`, execution `{path, result, latency_ms}`.
- Metrics (add later if infra available): counters for enqueue/flush, gauges for queue length and oldest event age, histogram for warm duration per path, errors.

## Config surface (new)
- `cache.event_idle_window_secs` (30–300, default 60).
- `cache.max_queue_events` (default 1024).
- `cache.warm_concurrency` (default 4).
- CLI/env wiring via `config/mod.rs`, `soffio.toml.example`.

## Low-intrusion integration strategy
- Provide a `CacheEventSink` handle injected where `ResponseCache` is today (HTTP state + job context). Emit events instead of calling helpers; no business logic changes.
- Keep existing `ResponseCache` API for read path; add `evict` and `put_override` if needed.
- Reuse `CacheWarmer` internals for rendering; move to accept dynamic path lists to avoid touching view/render code.

## Precision audit (code-linked)
- Homepage tag list rendering (`application::feed::build_tag_summaries`) uses `tag_filter_limit` and `order_tags_with_pins` (pinned first, then count desc, then name). Implications:
  - Tag name/pin changes may or may not affect homepage depending on slice membership; plan builder should compute inclusion using counts + settings, or conservatively warm homepage when uncertain.
  - Post publish/unpublish changes tag counts → can reshuffle which tags appear; `PostChanged` must refresh homepage tag list and affected tag pages.
- Month summaries respect `month_filter_limit`; `PostChanged` should warm month list and affected month page when published month counts change.
- Post cards/badges on homepage/tag/month pages embed tag names; tag rename requires warming any page that renders posts containing that tag, even if the tag itself is not pinned.
