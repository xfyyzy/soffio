# Rollout Plan — Evented Cache Prewarm (Clean Cutover)

Decision: no shadow or dual-run. We fully remove the legacy invalidate+warm pipeline before introducing the new evented prewarm consumer.

## Phase 0 — Policy alignment
- Remove AGENTS.md §0.7 synchronous-invalidation rule; document the new evented model (add after implementation).
- Announce new SLA (idle window bound) to stakeholders.

## Phase 1 — Legacy removal (clean base)
- Excise `invalidate_and_enqueue_warm`, `CacheWarmJobPayload`, `JobType::WarmCache`, worker registration, and admin/API middleware invalidation hooks.
- Strip WarmCache references from admin UI, API docs, changelog, tests (`live_api`, `cache_consistency`, job counts).
- Ensure the service still builds/tests without the legacy cache path (temporary: cache warming disabled until Phase 2).

## Phase 2 — New evented prewarm implementation
- Introduce `CacheEventQueue` + `CacheEventSink`, inject into HTTP/admin/job contexts.
- Implement `CacheSyncPlan` builder + `CacheWarmExecutor`; add `ResponseCache::evict` and reuse `CacheWarmer` with dynamic path list.
- Add config flags: `cache.event_idle_window_secs` (default 60, bounds 30–300), `cache.max_queue_events`, `cache.warm_concurrency`.
- Wire business emit points (API/admin writes, render completion, publish jobs, settings/nav/tag changes, snapshot rollback) to enqueue events.
- Ship helper modules (`TagSliceCalculator`, `MonthSliceCalculator`, `RouteBuilder`) and emit helper fns to keep code changes low-intrusion.

## Phase 3 — Validation and guardrails
- Add E2E covering enqueue→flush pipeline including delete/unpublish evict path and idle-window auto-trigger.
- Add structured logs/metrics for enqueue, queue length/age, plan stats, execution results.
- Document operational runbook in `docs/cache/` (SLA, knobs, troubleshooting) and update `README*`/`CHANGELOG.md`/`docs/api/openapi.yaml`.
