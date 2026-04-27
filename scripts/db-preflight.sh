#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

DEFAULT_SQLX_TEST_DATABASE_URL="postgres://soffio:soffio_local_dev@127.0.0.1:5432/postgres"
DEFAULT_DATABASE_URL="postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev"

: "${SQLX_TEST_DATABASE_URL:=${DEFAULT_SQLX_TEST_DATABASE_URL}}"
: "${DATABASE_URL:=${DEFAULT_DATABASE_URL}}"
export SQLX_TEST_DATABASE_URL DATABASE_URL

if [ "${SKIP_DB_PREFLIGHT:-0}" = "1" ]; then
  printf "==> DB preflight (skipped: SKIP_DB_PREFLIGHT=1)\n"
  exit 0
fi

if [ "${SQLX_TEST_DATABASE_URL}" != "${DEFAULT_SQLX_TEST_DATABASE_URL}" ] ||
  [ "${DATABASE_URL}" != "${DEFAULT_DATABASE_URL}" ]; then
  printf "==> DB preflight (skipped: custom DATABASE_URL/SQLX_TEST_DATABASE_URL)\n"
  exit 0
fi

if ! command -v docker >/dev/null 2>&1; then
  cat >&2 <<'EOF'
Database preflight failed: docker is required for the default local Postgres check.

Install Docker, point DATABASE_URL/SQLX_TEST_DATABASE_URL at an already running
Postgres instance, or rerun with SKIP_DB_PREFLIGHT=1 if you intentionally manage
the database outside this repository.
EOF
  exit 1
fi

if docker compose -f docker-compose-dev.yml exec -T postgres pg_isready -U soffio -d postgres >/dev/null 2>&1; then
  printf "==> DB preflight (ok: docker compose postgres is ready)\n"
  exit 0
fi

cat >&2 <<'EOF'
Database preflight failed: default local Postgres is not ready.

Run:
  docker compose -f docker-compose-dev.yml up -d

Then rerun the gate:
  ./scripts/gate-fast.sh
  ./scripts/gate-full.sh

For the full local live-test flow, use:
  ./scripts/gate-full-local.sh
EOF
exit 1
