# Soffio

[![CI](https://github.com/xfyyzy/soffio/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/xfyyzy/soffio/actions/workflows/ci.yml)
[![Release](https://github.com/xfyyzy/soffio/actions/workflows/release.yml/badge.svg)](https://github.com/xfyyzy/soffio/actions/workflows/release.yml)
[![Rust Edition](https://img.shields.io/badge/Rust%20Edition-2024-orange?logo=rust&logoColor=white)](https://doc.rust-lang.org/edition-guide/)
[![Public Site](https://img.shields.io/website?url=https%3A%2F%2Fsoffio.xfyyzy.xyz&label=public%20site)](https://soffio.xfyyzy.xyz)
[![Admin Site](https://img.shields.io/website?url=https%3A%2F%2Fadmin.soffio.xfyyzy.xyz&label=admin%20site)](https://admin.soffio.xfyyzy.xyz)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/xfyyzy/soffio)

English | [中文](README.zh.md)

Soffio is a calm, self-hosted publishing system for technical writers who want static output, admin convenience, and operational control.

Soffio is not a general-purpose CMS. It is a publishing system for people who want to write, publish, automate, and self-host without surrendering control. It favors static output over runtime magic, boring reliability over plugin sprawl, and explicit workflows over hidden automation.

The public site renders posts statically and keeps reader-facing interactivity server-driven, while the admin console focuses on writing, editing, automation, and release workflows. The stack centers on Rust, Axum, Askama, and SQLx, and the codebase enforces a domain/application/infra layering model (see `AGENTS.md`). Soffio is released under the BSD-2-Clause license.

## Demo Environments

- Public site: <https://soffio.xfyyzy.xyz>
- Admin site: <https://admin.soffio.xfyyzy.xyz>

The demo database resets at the top of every hour.

## Repository Layout

```
src/
├── domain        # domain entities, invariants, value objects
├── application   # use-case services, repo traits, job scheduling
├── infra         # Postgres repos, HTTP adapters, telemetry
├── presentation  # view models, templates, layouts
├── util          # supporting utilities (time zones, ids, etc.)
└── main.rs       # CLI / service entry point
```

## Prerequisites

- Rust stable ≥ 1.91 (2024 Edition ready)
- PostgreSQL 18 (default DSN `postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev`)
- TypeScript Compiler 6.x (`tsc --version`, validated with 6.0.2)

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
- **Telemetry** — `tracing` + `tracing-subscriber`, bootstrapped via `src/infra/telemetry.rs`.

## Headless API

- Base path: `/api/v1` on the public listener.
- Auth: `Authorization: Bearer <api_key>` (obtain/manage keys in the admin UI under “API keys”; keys are shown once). Admin workflow documented in [`docs/admin/api-keys.md`](docs/admin/api-keys.md).
- Scopes control access (snake_case): `post_read`, `post_write`, `page_read`, `page_write`, `tag_read`, `tag_write`, `navigation_read`, `navigation_write`, `upload_read`, `upload_write`, `settings_read`, `settings_write`, `job_read`, `audit_read`.
- Rate limit: configured via `api_rate_limit` (default: 120 requests per 60s per key).
- Specification: [`docs/api/openapi.yaml`](docs/api/openapi.yaml).

## soffio-cli

Headless API CLI for admins and automation. See [`docs/cli.md`](docs/cli.md) for the generated command matrix and full usage guide.

Quick start:

```
cargo build -p soffio-cli --release
SOFFIO_SITE_URL=https://your.site \
SOFFIO_API_KEY_FILE=~/.config/soffio/key \
./target/release/soffio-cli api-keys me
```

Create a post from files:

```
./target/release/soffio-cli posts create \
  --title "Title" --excerpt "Summary" \
  --body-file ./post.md --summary-file ./post.summary.md \
  --status published
```

## Development Workflow

1. Run the baseline quality gates:
   ```bash
   # point DATABASE_URL to your writable instance; SQLX_TEST_DATABASE_URL is used by `#[sqlx::test]` to create temp DBs
   export DATABASE_URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev
   export SQLX_TEST_DATABASE_URL=postgres://soffio:soffio_local_dev@127.0.0.1:5432/postgres

   # default fast loop
   ./scripts/gate-fast.sh

   # before PR/merge; starts local Postgres, imports seed data, renders derived content,
   # starts a temporary soffio instance, runs the full gate, then stops the instance
   ./scripts/gate-full-local.sh

   # use this only after manually preparing the database and local soffio instance
   ./scripts/gate-full.sh

   # periodic dependency hygiene (for example weekly)
   ./scripts/gate-hygiene.sh
   ```
2. To prepare the full gate prerequisites manually instead of using `gate-full-local.sh`, run:
   ```bash
   docker compose -f docker-compose-dev.yml up -d

   SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev \
     cargo run --bin soffio -- import seed/seed.toml

   SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev \
     cargo run --bin soffio -- renderall

   SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev \
     target/debug/soffio serve
   ```
   `gate-full.sh` fails fast when the default local Postgres container is not ready or the seeded API at
   `tests/api_keys.seed.toml` is unavailable. Set `SKIP_LIVE_TESTS=1` only for non-release diagnostics.
3. Consult `CONTRIBUTING.md` for branching strategy, commit format, and review expectations.
4. Follow `.github/PULL_REQUEST_TEMPLATE.md` and ensure CI stays green before merging.

## Deployment

Production deployments are typically containerized. Refer to [`docs/deploy/docker.md`](docs/deploy/docker.md) for compose files, environment variables, health checks, and operational tips.

## Releases & Changelog

- Every tagged release is documented in `CHANGELOG.md`.
- Each release should ship with:
  1. Migration scripts plus backward-compatibility notes.
  2. A list of new/changed configuration keys and their defaults.
  3. Linux musl archives and FreeBSD x86_64 archives when binary artifacts are published.

## Support, Community & Security

- Support channels and FAQs: `SUPPORT.md`
- Security disclosure process: `SECURITY.md`
- Code of Conduct: `CODE_OF_CONDUCT.md`

## License

BSD-2-Clause — see `LICENSE` for the full text.
