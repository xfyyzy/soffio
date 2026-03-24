#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

: "${SQLX_TEST_DATABASE_URL:=postgres://soffio:soffio_local_dev@127.0.0.1:5432/postgres}"
: "${DATABASE_URL:=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev}"
export SQLX_TEST_DATABASE_URL DATABASE_URL

printf "==> cargo fmt --all -- --check\n"
cargo fmt --all -- --check

printf "==> cargo check --workspace --all-targets\n"
cargo check --workspace --all-targets

printf "==> cargo clippy --workspace --all-targets -- -D warnings\n"
cargo clippy --workspace --all-targets -- -D warnings

printf "==> cargo nextest run --workspace --lib --cargo-quiet --no-tests warn\n"
cargo nextest run --workspace --lib --cargo-quiet --no-tests warn
