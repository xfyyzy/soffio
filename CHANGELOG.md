# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- CI and release workflows now explicitly use `--target` to enable Cargo's cross-compilation mode, ensuring build.rs and proc-macros use host default instruction set and are not polluted by target-specific CPU flags; fixes intermittent SIGILL errors when cached build artifacts run on different GitHub runner CPUs.

## [0.1.11] - 2025-12-07

### Fixed
- Admin API key scope picker now keeps selected scopes visible in the available grid (like the tag picker), preventing option reordering when toggling many scopes.
- Release workflow restricts target CPU flags to the musl target only, avoiding host build-script crashes when adding higher x86-64 levels.

### Added
- Release artifacts and Docker images now also target `x86-64-v4` alongside v2/v3.

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
