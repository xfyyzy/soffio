# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.12] - 2025-12-10

### Fixed
- **Render job race condition**: `RenderPostJobPayload` now carries `body_markdown` and `summary_markdown` inline instead of re-reading from the database. This prevents a race condition where the job worker (using a separate connection pool) could read stale data before the HTTP request's write was fully visible.
- **Jobs admin page alignment**: Fixed filter state loss during pagination and status tab switching. Added Search field for querying Payload and Last Error. Added missing `id` hidden field to row actions. Unified templates by removing Jobs-specific `status_tabs.html` in favor of generic template with `job_type_filter_enabled` support.
- **Audit log page fixes**: Fixed entity type tabs to show all types even with count=0. Fixed UUID column width (38ch for full UUIDs). Fixed filter state loss on pagination/tab switch via new generic `custom_hidden_fields` mechanism. Added detail page for viewing individual audit entries.

### Added
- **Jobs admin page**: New admin panel page at `/jobs` for viewing background task execution status. Features include status filter tabs (All/Pending/Running/Done/Failed/Killed), Job Type dropdown filter, bidirectional cursor pagination, and row actions (Retry/Cancel). Uses status badges for all enumerable types consistent with other admin pages.
- **Audit log admin page**: New admin panel page at `/audit` for viewing system audit logs. Features include Entity Type tabs with counts, Actor/Action dropdown filters, bidirectional cursor pagination, and detail page at `/audit/{id}`. Aligned with Posts page pattern using shared templates.
- Unit tests for `RenderPostJobPayload` serialization to ensure payload integrity.
- Integration test `live_api_post_body_renders_immediately` validating that body patches trigger immediate rendering with correct content.
- Documented async job payload architecture principle in AGENTS.md §5: job payloads should carry complete execution context to avoid cross-pool read inconsistencies.

### Changed
- **Admin templates refactoring**: Introduced generic `custom_hidden_fields` mechanism replacing hardcoded filter field conditionals in `status_tabs.html` and `pagination.html`. This follows Open-Closed Principle—adding new page types no longer requires modifying shared templates.

## [0.1.11] - 2025-12-08

### Fixed
- **API cache invalidation**: API routes (`/api/v1/*`) now correctly invalidate the public response cache after write operations, ensuring content modified via `soffio-cli` is immediately reflected on the public site. Previously, API writes did not trigger cache invalidation, causing stale content to be served.
- Removed redundant cache invalidation calls from service layer (`AdminPostService`, `AdminPageService`), keeping cache logic in the HTTP middleware where it belongs.
- CI and release workflows now explicitly use `--target` to enable Cargo's cross-compilation mode, ensuring build.rs and proc-macros use host default instruction set and are not polluted by target-specific CPU flags; fixes intermittent SIGILL errors when cached build artifacts run on different GitHub runner CPUs.
- Local development no longer forces cross-compilation target; `.cargo/config.toml` now only applies musl settings when `--target` is explicitly passed.
- **Mermaid SVG id collision**: Mermaid diagrams now render with unique SVG ids (`mermaid-{hash}`) instead of the default `my-svg`, preventing CSS/JS conflicts when multiple diagrams appear on the same page.
- Admin API key scope picker now keeps selected scopes visible in the available grid (like the tag picker), preventing option reordering when toggling many scopes.
- Release workflow restricts target CPU flags to the musl target only, avoiding host build-script crashes when adding higher x86-64 levels.

### Added
- Async cache warming: after API write operations, a `WarmCache` job is asynchronously enqueued to pre-warm commonly accessed pages (home, pinned tags, navigation pages). This maintains consistent user experience without blocking API responses.
- `CacheWarmDebouncer`: prevents redundant cache warming when multiple writes occur in quick succession (5-second debounce window).
- Cache consistency E2E tests (`live_api_cache_invalidation_on_update`, `live_api_cache_invalidation_on_page_update`) that verify public pages are updated immediately after API modifications.
- Release artifacts and Docker images now also target `x86-64-v4` alongside v2/v3.

### Changed
- AGENTS.md now requires English for all code comments, documentation, commit messages, and user-facing text.
- AGENTS.md now prohibits `git commit --amend` unless explicitly requested by the user.

## [0.1.10] - 2025-12-07
### Changed
- CI and release workflows now build and reuse lightweight builder images (with optional redocly via build-arg) to avoid per-run toolchain setup and speed up pipelines; no runtime code changes.

## [0.1.9] - 2025-12-05
### Fixed
- Prevented settings page textareas from overflowing their panels by using border-box sizing and block display within the settings summary text styles.
- Rendering now distinguishes internal vs external links using `public_site_url` (same-origin or relative count as internal) and forces external links to open in a new tab with `rel="noopener noreferrer"` to avoid `window.opener` risks; rendering stays pure by taking the site URL as input.

## [0.1.8] - 2025-12-04
### Breaking
- Renamed title patch endpoints: `POST /api/v1/posts/{id}/title-slug` → `POST /api/v1/posts/{id}/title`; `POST /api/v1/pages/{id}/title-slug` → `POST /api/v1/pages/{id}/title`. Slugs are now immutable after creation and cannot be provided in title patch payloads.
- CLI commands aligned: `posts patch-title-slug` / `pages patch-title-slug` replaced with `posts patch-title --id --title` and `pages patch-title --id --title`.

## [0.1.7] - 2025-12-04
### Added
- Added read endpoints for navigation and uploads (GET by id), plus read-by-id for posts/pages and read-by-id/slug for tags; CLI gains matching `get` subcommands for posts, pages, tags, navigation, and uploads.
- Regenerated OpenAPI spec and CLI docs to reflect the new read capabilities.

### Fixed
- Updated snapshots and static asset version query params to align with release 0.1.7.

## [0.1.0] - 2025-11-01

### Added

- Initial open-source release of Soffio with public/admin HTTP services, deterministic rendering pipeline.
- Comprehensive Postgres schema covering posts, pages, navigation, tags, site settings.
- Axum server binaries exposing `serve` and `renderall` `import` `export` commands.
