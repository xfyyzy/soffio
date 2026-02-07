# ADR: Module Decomposition Plan for Large Admin/Config Modules

- Status: Accepted
- Date: 2026-02-06
- Scope: Internal code organization only, no user-visible behavior changes

## Context

The following files have grown beyond a size that makes review, change isolation, and compile-impact reasoning harder:

- `src/application/admin/dashboard.rs`
- `src/config/mod.rs`
- `src/presentation/admin/views.rs`

Current pain points:

- Repeated patterns are difficult to modify consistently.
- File-level merge conflicts are frequent when unrelated edits land together.
- Rebuild fanout can be larger than needed because unrelated concerns share the same module.

## Decision

Decompose these modules into focused submodules while preserving existing public APIs.

The decomposition will be done in small, behavior-preserving slices:

1. Move pure helpers/constants first.
2. Move feature-specific view/config/application blocks next.
3. Keep existing re-export surface stable from the original module boundary.
4. Run existing integration/snapshot tests after each slice.

## Design Constraints

- No behavior change and no route/template/output drift.
- No schema or migration change.
- No public API break under `soffio` crate boundaries.
- No hidden runtime flags.

## Proposed Target Layout

### `src/application/admin/dashboard.rs`

Split into:

- `src/application/admin/dashboard/mod.rs` (public service/deps + orchestration)
- `src/application/admin/dashboard/panels/posts.rs`
- `src/application/admin/dashboard/panels/pages.rs`
- `src/application/admin/dashboard/panels/tags.rs`
- `src/application/admin/dashboard/panels/navigation.rs`
- `src/application/admin/dashboard/panels/uploads.rs`
- `src/application/admin/dashboard/panels/api_keys.rs`
- `src/application/admin/dashboard/error.rs` (repo failure mapping)

### `src/config/mod.rs`

Split into:

- `src/config/mod.rs` (public entrypoint and re-exports)
- `src/config/cli.rs` (clap structs and command wiring)
- `src/config/defaults.rs` (constants/default constructors)
- `src/config/loading.rs` (file/env loading and merge)
- `src/config/overrides.rs` (CLI override application)
- `src/config/types.rs` (settings structs)

### `src/presentation/admin/views.rs`

Split into:

- `src/presentation/admin/views/mod.rs` (shared helpers and exports)
- `src/presentation/admin/views/dashboard.rs`
- `src/presentation/admin/views/posts.rs`
- `src/presentation/admin/views/pages.rs`
- `src/presentation/admin/views/tags.rs`
- `src/presentation/admin/views/navigation.rs`
- `src/presentation/admin/views/settings.rs`
- `src/presentation/admin/views/uploads.rs`
- `src/presentation/admin/views/snapshots.rs`
- `src/presentation/admin/views/jobs.rs`

## Verification Strategy

For each extraction slice:

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --test admin_panels -- --nocapture`
4. `cargo test --test snapshots -- --nocapture`

Final pass after all slices:

- `cargo nextest run --workspace --all-targets`

## Consequences

Positive:

- Lower review complexity per change.
- Better ownership boundaries.
- Fewer conflict hotspots.

Tradeoffs:

- More files and module wiring.
- Refactor churn in the short term.

## Rollback

This change is organizational. If needed, slices can be reverted independently without data migration.
