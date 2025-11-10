# Docker Deployment

English | [中文](docker.zh.md)

This guide explains how to build and run production-ready Soffio images with `deploy/docker/Dockerfile`, along with runtime configuration tips, health checks, and common operations.

## Image Layout

- **Builder** — based on `rust:1.91-alpine3.20`, bundles `cargo-chef` and the TypeScript compiler, and produces a static binary via `TARGET_TRIPLE` (default `x86_64-unknown-linux-musl`) and `TARGET_CPU` (default `x86-64-v2`).
- **Runtime** — extends `MERMAID_CLI_IMAGE` (default `minlag/mermaid-cli:latest`), adds the `soffio` binary plus a Mermaid CLI wrapper script to support server-side diagram rendering.

## Build the Image

```bash
docker buildx build \
  --platform linux/amd64 \
  --build-arg TARGET_TRIPLE=x86_64-unknown-linux-musl \
  --build-arg TARGET_CPU=x86-64-v2 \
  --build-arg MERMAID_CLI_IMAGE=minlag/mermaid-cli:10.5.1 \
  -f deploy/docker/Dockerfile \
  -t soffio:latest \
  .
```

## Runtime Configuration

All settings are provided via environment variables (see `soffio.toml.example`). Frequently used options:

| Variable                                              | Description                  | Default / Example                                         |
|-------------------------------------------------------|------------------------------|-----------------------------------------------------------|
| `SOFFIO__DATABASE__URL`                               | Postgres connection string   | Required, e.g. `postgres://soffio:***@db:5432/soffio_prod` |
| `SOFFIO__SERVER__PUBLIC_PORT`                         | Public listener port         | `3000`                                                     |
| `SOFFIO__SERVER__ADMIN_PORT`                          | Admin listener port          | `3001`                                                     |
| `SOFFIO__SERVER__HOST` / `SOFFIO__SERVER__ADMIN_HOST` | Bind addresses               | `0.0.0.0` inside the image                                 |
| `SOFFIO__UPLOADS__DIRECTORY`                          | Persistent uploads path      | `/var/lib/soffio/uploads`                                  |
| `SOFFIO__LOGGING__JSON`                               | Emit structured JSON logs    | `false`                                                    |

Soffio runs database migrations automatically on startup (`PostgresRepositories::run_migrations`), so ensure the DB user has sufficient privileges.

## Run Example

```bash
docker run -d \
  --name soffio \
  -p 3000:3000 \
  -p 3001:3001 \
  -v soffio_uploads:/var/lib/soffio/uploads \
  -e RUST_LOG=info \
  -e SOFFIO__DATABASE__URL=postgres://soffio:soffio_prod@postgres:5432/soffio_prod \
  -e SOFFIO__SERVER__HOST=0.0.0.0 \
  -e SOFFIO__SERVER__ADMIN_HOST=0.0.0.0 \
  ghcr.io/xfyyzy/soffio:amd64-x86-64-v2
```

Health checks:

- Public site: `GET /_health/db`
- Admin site: `GET /_health/db` on the admin listener (`SOFFIO__SERVER__ADMIN_PORT`)

## Using Docker Compose

The repo ships a `docker-compose.yml` that provisions PostgreSQL plus the Soffio app:

```bash
docker compose -f docker-compose.yml up -d
```

Before deploying, make sure to:

- Point `SOFFIO__DATABASE__URL` at your production database.
- Mount `/var/lib/soffio/uploads` to production storage (the sample maps it to `./uploads`).
- If you need the admin surface exposed, publish port `3001` or place it behind an internal network.
