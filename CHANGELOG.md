# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
