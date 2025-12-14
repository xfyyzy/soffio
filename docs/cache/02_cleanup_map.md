# Cleanup Map — Remove Legacy Invalidate+Warm Pipeline

Goal: fully excise the synchronous invalidation + WarmCache job model before introducing the new evented prewarm consumer.

## Code paths to change/remove
- `src/application/jobs/cache.rs`: delete `invalidate_and_enqueue_warm`, `CacheWarmJobPayload`, job enqueue/handler; replace with event emit hooks (thin sink trait) or remove entirely once new consumer exists.
- `src/domain/types.rs`: retire `JobType::WarmCache` if the new design no longer uses apalis jobs; otherwise repurpose with new payload semantics.
- `src/application/jobs/mod.rs` exports; `src/main.rs` worker registration (`cache-warm-worker`), `warm_initial_cache` startup call if superseded by new consumer.
- `src/infra/cache.rs`: keep cache store/get helpers but drop `CacheWarmDebouncer` if unused; re-evaluate `invalidate_all` semantics once “no invalidation” is enforced.
- HTTP middleware
  - `src/infra/http/middleware.rs`: remove `invalidate_and_warm_cache` / `invalidate_admin_writes` behavior; replace with cache-event emitters (non-blocking) wired to new queue.
  - `src/infra/http/api/mod.rs`, `src/infra/http/admin/mod.rs`: adjust layering/state wiring accordingly.
- Workers/jobs that call the helper today
  - `src/application/render/jobs.rs` (post render completion)
  - `src/application/jobs/publish.rs` (publish post/page jobs)
- Cache warmer logic
  - `src/infra/cache_warmer.rs`: may be reused for warming execution, but needs refactor to accept a set of paths/entries instead of fixed “warm everything”.
- Admin API & UI plumbing referencing warm-cache jobs
  - `src/application/admin/jobs.rs` counts `warm_cache`
  - `static/admin/app/20-components.css` badge styling
  - `docs/api/openapi.yaml` enumerates job type

## Database cleanup (before new design lands)
- `apalis.jobs` rows with `job_type IN ('warm_cache', 'invalidate_cache')` must be deleted to avoid orphaned jobs once handlers are removed. Add an idempotent migration (or ops SQL runbook) to purge these rows.
- If monitoring/analytics tables reference `warm_cache` job_type (e.g., materialized views or dashboards), drop/update them in the same migration or ops step.
- No cache state is persisted (in-memory only), so no cache table cleanup is needed.

## Tests to update/remove
- `tests/cache_consistency.rs`: expectations about synchronous invalidation and behavior of `invalidate_all()`.
- `tests/live_api.rs`: E2E assumes immediate invalidation; warm job count helpers will break if job type removed.
- `tests/api.rs` and any unit tests using `ResponseCache::invalidate_all` semantics.

## Docs/changelog/policy
- `AGENTS.md` §0.7 cache discipline: remove/replace with new constraints once design is accepted.
- `CHANGELOG.md` entries describing synchronous invalidation / warm job flow.
- `README.md`, `README.zh.md` sections that point to `cache_warmer.rs` + old behavior.
- Any admin/API docs referencing warm cache jobs (`docs/api/openapi.yaml`).

## Config & operational hooks
- Validate/repurpose `cache.enable_response_cache` flag (currently unused) to gate new cache service.
- Add new settings for event-queue auto-flush window, max batch size, and per-flush warm concurrency; wire into CLI/env (`config/mod.rs`, `soffio.toml.example`).
- Observe migrations: no DB schema change is expected for in-memory queue; avoid introducing new tables unless later required.
