# Soffio Optimization Roadmap Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Improve API stability, cache correctness/latency, dashboard efficiency, and operational observability while preserving current public behavior and architecture boundaries.

**Architecture:** The plan keeps domain invariants unchanged and applies optimizations at adapter/infra boundaries first. Runtime behavior changes are isolated behind typed config and verified by unit, integration, and live tests. Database changes are additive (index-only) and backward compatible.

**Tech Stack:** Rust 2024, Axum 0.8, SQLx 0.8 (Postgres), apalis jobs, tracing, cargo-nextest, sqlx::test, Docker Compose Postgres.

## Priority Summary

| Priority | Optimization Direction | Expected Benefit | Risk |
|---|---|---|---|
| P0 | API rate limiter algorithm + route-template keying | Lower CPU/memory under burst traffic, fairer limits | Medium |
| P0 | Dashboard aggregate SQL (remove pagination scans) | Lower DB round-trips and faster admin dashboard | Low |
| P1 | Cache invalidation/warming split (write path) | Lower write tail latency, same consistency guarantees | Medium |
| P1 | Cache event queue backpressure/coalescing | Prevent unbounded queue growth during bursts | Medium |
| P1 | Cache metrics (hit/miss/evict/queue/consume/warm) | Better operability and regression detection | Medium |
| P2 | Upload query indexes | Better list/count/filter performance at scale | Low |
| P2 | build.rs incremental asset pipeline | Faster local edit-compile cycle | Medium |
| P3 | Module decomposition for compile-time ergonomics | Lower cognitive load and build fanout long-term | Medium |

## Execution Status (2026-02-06)

| Task | Status | Notes |
|---|---|---|
| Task 1: Bounded-memory API rate limiter | Done | Token-bucket + stale bucket cleanup + unit tests added. |
| Task 2: Route-template rate-limit keying | Done | Middleware now prefers `MatchedPath`; integration test added. |
| Task 3: Dashboard aggregate SQL | Done | Added aggregate repo methods and integration tests. |
| Task 4: Cache invalidation/warm split | Done | Immediate write path uses invalidate-only; background loop uses full mode. |
| Task 5: Bounded cache event queue | Done | Added queue length cap + drop counter + config/CLI wiring. |
| Task 6: Cache metrics instrumentation | Done | Added L0/L1/event/consume/warm metrics + integration tests. |
| Task 7: Upload indexes migration | Done | Additive migration + index presence test + seed reconcile done. |
| Task 8: Incremental `build.rs` asset pipeline | Done | Fingerprint+stamp skip logic + dedicated tests added. |
| Task 9: Module decomposition slices | Done | Completed `application/admin/dashboard` panel collector extraction; fully decomposed `presentation/admin/views` into focused submodules (dashboard/navigation/pages/posts/settings/snapshots/tags/toast/uploads/editors/api_keys/jobs/audit); decomposed `config/mod.rs` by extracting CLI definitions to `config/cli.rs` and tests to `config/tests.rs`; behavior unchanged and regression suites green. |

## Non-Negotiable Invariants

- Keep domain model semantics unchanged (`src/domain/**`).
- No new `unsafe` or FFI.
- No hidden behavior behind undocumented env flags.
- Keep SQLx compile-time checks and regenerate `.sqlx` metadata if query shape changes.
- Keep request-path correctness: writes must still invalidate stale cache entries before response returns.

## Global Verification Gate (run after each task block)

```bash
export SQLX_TEST_DATABASE_URL=postgres://soffio:soffio_local_dev@127.0.0.1:5432/postgres
export DATABASE_URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev

cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace --all-targets
cargo test --test live_api --test live_cache -- --ignored --test-threads=1
cargo +nightly udeps --all-targets --workspace
cargo outdated -wR
```

Expected: all commands pass except `cargo outdated -wR` (report-only).

## Task-by-Task Plan

### Task 1 (P0): Replace API rate limiter algorithm with bounded-memory implementation

**Files:**
- Modify: `src/infra/http/api/rate_limit.rs`
- Test: `src/infra/http/api/rate_limit.rs` (unit tests module)

**Step 1: Write the failing test**

```rust
#[test]
fn limiter_memory_is_bounded_per_bucket() {
    let limiter = ApiRateLimiter::new(Duration::from_secs(60), 3);
    for _ in 0..10_000 {
        let _ = limiter.allow("key", "route");
    }
    assert!(limiter.debug_bucket_len("key", "route") <= 3);
}
```

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p soffio limiter_memory_is_bounded_per_bucket -- src/infra/http/api/rate_limit.rs
```

Expected: FAIL because current vector-retain implementation is unbounded in per-request work and lacks `debug_bucket_len`.

**Step 3: Write minimal implementation**

Implement a token-bucket or GCRA state per `(principal, route_template)` with constant-size state:

```rust
struct BucketState {
    tokens: f64,
    last_refill: Instant,
}
```

Add opportunistic stale-bucket cleanup to cap total map growth.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p soffio rate_limit -- src/infra/http/api/rate_limit.rs
```

Expected: PASS; include tests for allow/deny boundaries and retry-after behavior.

**Step 5: Commit**

```bash
git add src/infra/http/api/rate_limit.rs
git commit -m "perf(api): use bounded-memory rate limiter"
```

---

### Task 2 (P0): Use route template keying in API rate-limit middleware

**Files:**
- Modify: `src/infra/http/api/middleware.rs`
- Test: `tests/api.rs`

**Step 1: Write the failing integration test**

Add a test that hits two different resource IDs under the same route template and expects shared quota:

```rust
#[sqlx::test(migrations = "./migrations")]
async fn api_rate_limit_uses_route_template(pool: PgPool) {
    // build state + token
    // call handler for /api/v1/posts/{id} twice with different ids
    // assert second call decrements same bucket and headers reflect shared remaining
}
```

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test api api_rate_limit_uses_route_template -- --nocapture
```

Expected: FAIL because current middleware keys on raw `request.uri().path()`.

**Step 3: Write minimal implementation**

Extract `MatchedPath` and fallback to URI path only when missing:

```rust
let route_key = request
    .extensions()
    .get::<axum::extract::MatchedPath>()
    .map(|m| m.as_str().to_owned())
    .unwrap_or_else(|| request.uri().path().to_owned());
```

**Step 4: Run tests to verify pass**

Run:

```bash
cargo test --test api api_rate_limit_uses_route_template -- --nocapture
cargo test -p soffio rate_limit -- src/infra/http/api/rate_limit.rs
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/infra/http/api/middleware.rs tests/api.rs
git commit -m "fix(api): rate limit by matched route template"
```

---

### Task 3 (P0): Replace dashboard scan loops with aggregate repo queries

**Files:**
- Modify: `src/application/repos.rs`
- Modify: `src/infra/db/navigation.rs`
- Modify: `src/infra/db/uploads.rs`
- Modify: `src/application/admin/dashboard.rs`
- Create: `tests/admin_dashboard_metrics.rs`

**Step 1: Write the failing tests**

Create integration tests for aggregate correctness:

```rust
#[sqlx::test(migrations = "./migrations")]
async fn dashboard_navigation_external_count_matches_seed(pool: PgPool) {
    // seed internal + external links
    // assert aggregate external count is exact
}

#[sqlx::test(migrations = "./migrations")]
async fn dashboard_upload_total_bytes_matches_seed(pool: PgPool) {
    // seed uploads with known sizes
    // assert aggregate sum equals expected u64
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test --test admin_dashboard_metrics -- --nocapture
```

Expected: FAIL because trait methods do not exist yet.

**Step 3: Write minimal implementation**

- Add repo methods:
  - `NavigationRepo::count_external_navigation(...)`
  - `UploadsRepo::sum_upload_sizes(...)`
- Implement SQL aggregates with SQLx compile-time checking where possible.
- Remove `count_external_navigation` and `sum_upload_sizes` loop helpers from dashboard service.

**Step 4: Verify pass**

```bash
cargo test --test admin_dashboard_metrics -- --nocapture
cargo test --test admin_panels -- --nocapture
```

Expected: PASS; dashboard panel snapshots unchanged except timing behavior.

**Step 5: Commit**

```bash
git add src/application/repos.rs src/infra/db/navigation.rs src/infra/db/uploads.rs src/application/admin/dashboard.rs tests/admin_dashboard_metrics.rs
git commit -m "perf(admin): replace dashboard scan loops with SQL aggregates"
```

---

### Task 4 (P1): Split cache trigger path into immediate invalidation and deferred warming

**Files:**
- Modify: `src/cache/consumer.rs`
- Modify: `src/cache/trigger.rs`
- Modify: `src/main.rs`
- Modify: `src/cache/config.rs`
- Test: `src/cache/trigger.rs`
- Test: `src/cache/consumer.rs`
- Test: `tests/live_cache.rs`

**Step 1: Write failing tests**

Add unit tests proving write-path trigger only invalidates synchronously:

```rust
#[tokio::test]
async fn trigger_consume_now_skips_warm_phase() {
    // queue event, trigger consume_now
    // assert invalidation occurred
    // assert no warm repo call executed
}
```

Add live test assertion that post-update remains immediately visible (existing live cache tests should still pass).

**Step 2: Run tests to verify fail**

```bash
cargo test -p soffio trigger_consume_now_skips_warm_phase -- src/cache/trigger.rs
```

Expected: FAIL because current `consume()` always includes warm when warm actions exist.

**Step 3: Write minimal implementation**

- Introduce explicit consumer modes:
  - `consume_invalidate_only()`
  - `consume_full()`
- `CacheTrigger::trigger(..., consume_now=true)` uses invalidate-only mode.
- Background timer in `main.rs` runs full mode.

**Step 4: Verify pass**

```bash
cargo test -p soffio cache::trigger -- --nocapture
cargo test -p soffio cache::consumer -- --nocapture
cargo test --test live_cache -- --ignored --test-threads=1
```

Expected: PASS; write consistency preserved, warm work shifted off request tail.

**Step 5: Commit**

```bash
git add src/cache/consumer.rs src/cache/trigger.rs src/main.rs src/cache/config.rs
git commit -m "perf(cache): split sync invalidation from async warming"
```

---

### Task 5 (P1): Add bounded queue + drop/coalesce policy for cache events

**Files:**
- Modify: `src/cache/events.rs`
- Modify: `src/cache/config.rs`
- Modify: `src/config/mod.rs`
- Modify: `soffio.toml.example`
- Test: `src/cache/events.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn queue_respects_max_len_and_drops_oldest() {
    let q = EventQueue::new_with_limit(3);
    for i in 0..10 { q.publish(test_event(i)); }
    assert_eq!(q.len(), 3);
    assert_eq!(q.dropped_count(), 7);
}
```

**Step 2: Run to verify fail**

```bash
cargo test -p soffio queue_respects_max_len_and_drops_oldest -- src/cache/events.rs
```

Expected: FAIL because queue has no max limit/dropped counter.

**Step 3: Implement minimal behavior**

- Add config: `cache.max_event_queue_len` (default conservative).
- Add queue policy: `drop_oldest` (default).
- Add optional in-place coalesce for same entity/key where safe.

**Step 4: Verify pass**

```bash
cargo test -p soffio cache::events -- --nocapture
```

Expected: PASS with deterministic queue behavior.

**Step 5: Commit**

```bash
git add src/cache/events.rs src/cache/config.rs src/config/mod.rs soffio.toml.example
git commit -m "feat(cache): add bounded event queue with drop policy"
```

---

### Task 6 (P1): Add cache-focused metrics instrumentation

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/infra/telemetry.rs`
- Modify: `src/cache/middleware.rs`
- Modify: `src/cache/store.rs`
- Modify: `src/cache/events.rs`
- Modify: `src/cache/consumer.rs`
- Create: `tests/cache_metrics.rs`

**Step 1: Write failing tests**

```rust
#[tokio::test]
async fn cache_paths_emit_expected_metric_keys() {
    // install test recorder
    // execute hit/miss/invalidate flow
    // assert metric names and labels exist
}
```

**Step 2: Run to verify fail**

```bash
cargo test --test cache_metrics -- --nocapture
```

Expected: FAIL because metrics are not emitted yet.

**Step 3: Minimal implementation**

Emit counters/histograms:
- `soffio_cache_l0_hit_total`, `soffio_cache_l0_miss_total`, `soffio_cache_l0_evict_total`
- `soffio_cache_l1_hit_total`, `soffio_cache_l1_miss_total`, `soffio_cache_l1_evict_total`
- `soffio_cache_event_queue_len`, `soffio_cache_event_dropped_total`
- `soffio_cache_consume_ms`, `soffio_cache_warm_ms`

**Step 4: Verify pass**

```bash
cargo test --test cache_metrics -- --nocapture
cargo test -p soffio cache::middleware -- --nocapture
cargo test -p soffio cache::consumer -- --nocapture
```

Expected: PASS; metrics names stable and documented.

**Step 5: Commit**

```bash
git add Cargo.toml src/infra/telemetry.rs src/cache/middleware.rs src/cache/store.rs src/cache/events.rs src/cache/consumer.rs tests/cache_metrics.rs
git commit -m "feat(observability): add cache metrics and timings"
```

---

### Task 7 (P2): Add upload query indexes (additive migration)

**Files:**
- Create: `migrations/20260206130000_add_upload_query_indexes.up.sql`
- Create: `migrations/20260206130000_add_upload_query_indexes.down.sql`
- Modify: `seed/seed.toml` (via reconcile command)
- Test: `tests/db_indexes.rs`

**Step 1: Write failing test**

```rust
#[sqlx::test(migrations = "./migrations")]
async fn upload_indexes_exist(pool: PgPool) {
    // query pg_indexes for expected index names
    // assert all expected indexes present
}
```

Expected indexes:
- `uploads_created_at_id_idx`
- `uploads_content_type_created_at_id_idx`
- optional trigram index for filename search (if extension policy allows).

**Step 2: Run test to verify fail**

```bash
cargo test --test db_indexes upload_indexes_exist -- --nocapture
```

Expected: FAIL before migration.

**Step 3: Implement migration + reconcile**

```bash
cargo sqlx prepare --workspace --database-url postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev -- --all-targets
SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev cargo run --bin soffio migrations reconcile seed/seed.toml
```

**Step 4: Verify pass**

```bash
cargo test --test db_indexes upload_indexes_exist -- --nocapture
```

Expected: PASS.

**Step 5: Commit**

```bash
git add migrations/20260206130000_add_upload_query_indexes.up.sql migrations/20260206130000_add_upload_query_indexes.down.sql seed/seed.toml tests/db_indexes.rs .sqlx
git commit -m "perf(db): add upload query indexes"
```

---

### Task 8 (P2): Make build.rs asset preparation incremental

**Files:**
- Modify: `build.rs`
- Create: `tests/build_script_incremental.rs` (if practical) or `build.rs` unit tests

**Step 1: Write failing test(s)**

```rust
#[test]
fn fingerprint_changes_only_when_relevant_inputs_change() {
    // create temp tree
    // compute fingerprint
    // touch irrelevant file
    // assert unchanged
}
```

**Step 2: Run to verify fail**

```bash
cargo test -p soffio fingerprint_changes_only_when_relevant_inputs_change
```

Expected: FAIL because no fingerprint/stamp mechanism exists.

**Step 3: Implement minimal incremental logic**

- Introduce input fingerprinting for `static/**`, `frontend/ts/**`, `tsconfig.json`.
- Skip expensive copy/concat/tsc when fingerprint unchanged.
- Preserve deterministic output and rerun-if-changed semantics.

**Step 4: Verify pass**

```bash
cargo test -p soffio build_script_incremental -- --nocapture
cargo check --workspace --all-targets
```

Expected: PASS.

**Step 5: Commit**

```bash
git add build.rs tests/build_script_incremental.rs
git commit -m "perf(build): add incremental asset preparation in build script"
```

---

### Task 9 (P3): Decompose large modules with zero behavior change

**Files:**
- Modify: `src/application/admin/dashboard.rs`
- Modify: `src/config/mod.rs`
- Modify: `src/presentation/admin/views.rs`
- Create: `docs/adr/2026-02-06-module-decomposition-plan.md`
- Test: existing suites only (no new behavior tests required)

**Step 1: Write safety tests first (snapshot and behavior lock-in)**

```bash
cargo test --test admin_panels -- --nocapture
cargo test --test snapshots -- --nocapture
```

Capture baseline snapshots before refactor.

**Step 2: Refactor in small slices**

- Extract private submodules without changing public signatures.
- Keep each extraction compile-green.

**Step 3: Verify no behavior change**

```bash
cargo nextest run --workspace --all-targets
```

Expected: PASS with no snapshot drift unless intentional formatting-only changes.

**Step 4: Commit per slice**

```bash
git add src/application/admin/dashboard.rs
git commit -m "refactor(admin): split dashboard service into focused modules"
```

Repeat for `config` and `presentation/admin/views` in separate commits.

---

## Integration and Live Test Matrix

| Area | Unit Tests | Integration Tests | Live Tests |
|---|---|---|---|
| Rate limiter | `src/infra/http/api/rate_limit.rs` | `tests/api.rs` (`api_rate_limit_uses_route_template`) | N/A |
| Dashboard aggregates | query helper tests in repo modules | `tests/admin_dashboard_metrics.rs` | Optional manual admin latency check |
| Cache write path split | `src/cache/trigger.rs`, `src/cache/consumer.rs` | `tests/api.rs` write-after-read freshness checks | `tests/live_cache.rs` |
| Queue backpressure | `src/cache/events.rs` | N/A | `tests/live_cache.rs` (burst scenario optional) |
| Cache metrics | `tests/cache_metrics.rs` | cache middleware integration tests | log/metric scrape during live tests |
| Upload indexes | N/A | `tests/db_indexes.rs` | N/A |
| build.rs incremental | build helper unit tests | N/A | local repeated `cargo check` timing comparison |

## Recommended Commit Sequence

1. `perf(api): use bounded-memory rate limiter`
2. `fix(api): rate limit by matched route template`
3. `perf(admin): replace dashboard scan loops with SQL aggregates`
4. `perf(cache): split sync invalidation from async warming`
5. `feat(cache): add bounded event queue with drop policy`
6. `feat(observability): add cache metrics and timings`
7. `perf(db): add upload query indexes`
8. `perf(build): add incremental asset preparation in build script`
9. `refactor(*): module decomposition slices`

## Versioning and Release Strategy Assessment

### Recommendation

Use an **alpha train** before stable.

### Why alpha is justified

- Runtime behavior changes in request-critical paths:
  - API rate limiting semantics
  - cache invalidation/warming execution timing
- Operational changes require soak validation:
  - queue backpressure and drop policy
  - new metrics and dashboards
- Database migration is additive but still deployment-sensitive (index creation time on large tables).

### Proposed release progression

1. `0.1.16-alpha.1`
- Tasks 1-3 only (rate limiter + dashboard aggregates)
- Goal: validate API fairness and dashboard latency.

2. `0.1.16-alpha.2`
- Tasks 4-6 (cache behavior + queue + metrics)
- Goal: validate write tail latency and cache consistency under load.

3. `0.1.16-alpha.3`
- Tasks 7-8 (DB indexes + build pipeline)
- Goal: validate migration safety and developer workflow gains.

4. `0.1.16` stable
- After at least one full CI cycle + live tests green + no regressions from alpha soak.

### Exit criteria from alpha to stable

- No cache consistency failures in `tests/live_cache.rs`.
- No API rate-limit regressions in integration tests.
- No migration rollback issues in staging.
- Observability confirms expected trends: lower dashboard latency, bounded queue length, acceptable drop count (ideally zero in nominal load).

## Changelog Discipline

- Add every merged task to `CHANGELOG.md` under **Unreleased** with user-visible impact.
- Mark breaking changes explicitly (none expected in this plan).
- When promoting stable, move alpha entries into `0.1.16` section with concise release notes.

## Execution Notes for Next Step

- Implement tasks in strict priority order.
- Run the global verification gate after each task.
- Stop and escalate immediately if invariant risk appears (public API shape, cache correctness, or migration safety).
