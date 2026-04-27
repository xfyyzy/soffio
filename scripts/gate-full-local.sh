#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

DEFAULT_SQLX_TEST_DATABASE_URL="postgres://soffio:soffio_local_dev@127.0.0.1:5432/postgres"
DEFAULT_DATABASE_URL="postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev"

: "${SQLX_TEST_DATABASE_URL:=${DEFAULT_SQLX_TEST_DATABASE_URL}}"
: "${DATABASE_URL:=${DEFAULT_DATABASE_URL}}"
: "${SOFFIO__DATABASE__URL:=${DATABASE_URL}}"
: "${SOFFIO_LIVE_WAIT_SECONDS:=60}"
export SQLX_TEST_DATABASE_URL DATABASE_URL SOFFIO__DATABASE__URL

server_pid=""

cleanup() {
  if [ -n "${server_pid}" ] && kill -0 "${server_pid}" >/dev/null 2>&1; then
    printf "==> stopping local soffio server (pid %s)\n" "${server_pid}"
    kill "${server_pid}" >/dev/null 2>&1 || true
    wait "${server_pid}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT HUP INT TERM

if [ "${SKIP_LIVE_TESTS:-0}" = "1" ]; then
  printf "==> live environment setup (skipped: SKIP_LIVE_TESTS=1)\n"
  ./scripts/gate-full.sh "$@"
  exit 0
fi

if [ "${SQLX_TEST_DATABASE_URL}" = "${DEFAULT_SQLX_TEST_DATABASE_URL}" ] &&
  [ "${DATABASE_URL}" = "${DEFAULT_DATABASE_URL}" ] &&
  [ "${SKIP_DB_PREFLIGHT:-0}" != "1" ]; then
  printf "==> docker compose -f docker-compose-dev.yml up -d\n"
  docker compose -f docker-compose-dev.yml up -d

  printf "==> waiting for local Postgres\n"
  db_waited=0
  while ! ./scripts/db-preflight.sh >/dev/null 2>&1; do
    db_waited=$((db_waited + 1))
    if [ "${db_waited}" -ge "${SOFFIO_LIVE_WAIT_SECONDS}" ]; then
      ./scripts/db-preflight.sh || true
      exit 1
    fi
    sleep 1
  done
  ./scripts/db-preflight.sh
else
  printf "==> local Postgres startup (skipped: custom DB env or SKIP_DB_PREFLIGHT=1)\n"
fi

if ./scripts/live-preflight.sh >/dev/null 2>&1; then
  cat >&2 <<'EOF'
Live test preflight found an already running soffio API.

Stop the existing local server before using ./scripts/gate-full-local.sh so this
wrapper can seed the database, start a fresh process, and clean it up reliably.
If you intentionally want to reuse the existing server, run ./scripts/gate-full.sh
directly instead.
EOF
  exit 1
fi

printf "==> cargo build --bin soffio\n"
cargo build --bin soffio

printf "==> import seed/seed.toml\n"
target/debug/soffio import seed/seed.toml

printf "==> renderall\n"
target/debug/soffio renderall

printf "==> starting local soffio server\n"
target/debug/soffio serve &
server_pid="$!"

printf "==> waiting for local soffio API\n"
server_waited=0
while ! ./scripts/live-preflight.sh >/dev/null 2>&1; do
  if ! kill -0 "${server_pid}" >/dev/null 2>&1; then
    wait "${server_pid}" >/dev/null 2>&1 || true
    printf "Local soffio server exited before becoming ready.\n" >&2
    exit 1
  fi

  server_waited=$((server_waited + 1))
  if [ "${server_waited}" -ge "${SOFFIO_LIVE_WAIT_SECONDS}" ]; then
    ./scripts/live-preflight.sh || true
    exit 1
  fi
  sleep 1
done
./scripts/live-preflight.sh

./scripts/gate-full.sh "$@"
