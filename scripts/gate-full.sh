#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

: "${SQLX_TEST_DATABASE_URL:=postgres://soffio:soffio_local_dev@127.0.0.1:5432/postgres}"
: "${DATABASE_URL:=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev}"
export SQLX_TEST_DATABASE_URL DATABASE_URL

./scripts/db-preflight.sh

printf "==> cargo fmt --all -- --check\n"
cargo fmt --all -- --check

printf "==> cargo check --workspace --all-targets\n"
cargo check --workspace --all-targets

printf "==> cargo clippy --workspace --all-targets -- -D warnings\n"
cargo clippy --workspace --all-targets -- -D warnings

printf "==> ./scripts/nextest-full.sh --no-fail-fast\n"
./scripts/nextest-full.sh --no-fail-fast

if [ "${SKIP_LIVE_TESTS:-0}" = "1" ]; then
  printf "==> live tests (skipped: SKIP_LIVE_TESTS=1)\n"
else
  ./scripts/live-preflight.sh

  printf "==> cargo test --test live_api --test live_cache -- --ignored --test-threads=1\n"
  cargo test --test live_api --test live_cache -- --ignored --test-threads=1
fi

if [ "${RUN_FEATURE_POWERSET:-0}" = "1" ]; then
  printf "==> cargo hack test --workspace --feature-powerset --depth 1\n"
  cargo hack test --workspace --feature-powerset --depth 1
else
  printf "==> cargo hack feature powerset (skipped: set RUN_FEATURE_POWERSET=1)\n"
fi
