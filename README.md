# Soffio

[![CI](https://github.com/xfyyzy/soffio/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/xfyyzy/soffio/actions/workflows/ci.yml)
[![Release](https://github.com/xfyyzy/soffio/actions/workflows/release.yml/badge.svg)](https://github.com/xfyyzy/soffio/actions/workflows/release.yml)
[![Rust Edition](https://img.shields.io/badge/Rust%20Edition-2024-orange?logo=rust&logoColor=white)](https://doc.rust-lang.org/edition-guide/)
[![Public Site](https://img.shields.io/website?url=https%3A%2F%2Fsoffio.xfyyzy.xyz&label=public%20site)](https://soffio.xfyyzy.xyz)
[![Admin Site](https://img.shields.io/website?url=https%3A%2F%2Fadmin.soffio.xfyyzy.xyz&label=admin%20site)](https://admin.soffio.xfyyzy.xyz)

English | [中文](README.zh.md)

Soffio is a Rust-powered publishing platform. The public site renders posts statically and sprinkles interactive widgets, while the admin console focuses on writing, editing, and releasing content. The stack centers on Axum, Askama, and SQLx, and the codebase enforces a domain/application/infra layering model (see `AGENTS.md`). Soffio is released under the BSD-2-Clause license.

## Demo Environments

- Public site: <https://soffio.xfyyzy.xyz>
- Admin site: <https://admin.soffio.xfyyzy.xyz>

The demo database resets at the top of every hour.

## Repository Layout

```
src/
├── domain        # domain entities, invariants, value objects
├── application   # use-case services, repo traits, job scheduling
├── infra         # Postgres repos, HTTP adapters, cache, telemetry
├── presentation  # view models, templates, layouts
├── util          # supporting utilities (time zones, ids, etc.)
└── main.rs       # CLI / service entry point
```

## Prerequisites

- Rust stable ≥ 1.91 (2024 Edition ready)
- PostgreSQL 18 (default DSN `postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev`)
- TypeScript Compiler 5.9.3

## Quick Start

1. Install the prerequisites above and create the `soffio_dev` database.
2. Launch the service:
   ```bash
   SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev cargo run --bin soffio
   ```
3. Browse the defaults:
   - Public site at `http://127.0.0.1:3000`
   - Admin site at `http://127.0.0.1:3001`
   - Override addresses via CLI flags or environment variables when needed.

## Runtime Components

- **HTTP services** — Axum 0.8 with separate listeners for public and admin traffic (`src/infra/http/public.rs` and `src/infra/http/admin/`).
- **Database access** — SQLx (Postgres); concrete repos live in `src/infra/db`, while traits are defined in `src/application/repos.rs`.
- **Caching** — response cache at `src/infra/cache.rs` plus a warmer in `src/infra/cache_warmer.rs`.
- **Telemetry** — `tracing` + `tracing-subscriber`, bootstrapped via `src/infra/telemetry.rs`.

## Development Workflow

1. Run the baseline quality gates:
   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --all-targets
   ```
2. Consult `CONTRIBUTING.md` for branching strategy, commit format, and review expectations.
3. Follow `.github/PULL_REQUEST_TEMPLATE.md` and ensure CI stays green before merging.

## Deployment

Production deployments are typically containerized. Refer to [`docs/deploy/docker.md`](docs/deploy/docker.md) for compose files, environment variables, health checks, and operational tips.

## Releases & Changelog

- Every tagged release is documented in `CHANGELOG.md`.
- Each release should ship with:
  1. Migration scripts plus backward-compatibility notes.
  2. A list of new/changed configuration keys and their defaults.

## Support, Community & Security

- Support channels and FAQs: `SUPPORT.md`
- Security disclosure process: `SECURITY.md`
- Code of Conduct: `CODE_OF_CONDUCT.md`

## License

BSD-2-Clause — see `LICENSE` for the full text.
