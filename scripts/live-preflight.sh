#!/usr/bin/env sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

SEED_KEYS_FILE="tests/api_keys.seed.toml"

if [ "${SKIP_LIVE_TESTS:-0}" = "1" ]; then
  printf "==> live preflight (skipped: SKIP_LIVE_TESTS=1)\n"
  exit 0
fi

if ! command -v curl >/dev/null 2>&1; then
  cat >&2 <<'EOF'
Live test preflight failed: curl is required to check the local soffio API.
EOF
  exit 1
fi

if [ ! -f "${SEED_KEYS_FILE}" ]; then
  printf "Live test preflight failed: missing %s\n" "${SEED_KEYS_FILE}" >&2
  exit 1
fi

base_url="$(
  sed -n 's/^base_url[[:space:]]*=[[:space:]]*"\(.*\)"[[:space:]]*$/\1/p' "${SEED_KEYS_FILE}" |
    sed -n '1p'
)"

api_key="$(
  awk '
    /^\[keys\]$/ { in_keys = 1; next }
    /^\[/ { in_keys = 0 }
    in_keys && /^all[[:space:]]*=/ {
      value = $0
      sub(/^[^=]*=[[:space:]]*"/, "", value)
      sub(/"[[:space:]]*$/, "", value)
      print value
      exit
    }
  ' "${SEED_KEYS_FILE}"
)"

if [ -z "${base_url}" ] || [ -z "${api_key}" ]; then
  printf "Live test preflight failed: unable to read base_url and keys.all from %s\n" "${SEED_KEYS_FILE}" >&2
  exit 1
fi

base_url="${base_url%/}"
probe_url="${base_url}/api/v1/api-keys/me"

if curl -fs --connect-timeout 2 --max-time 5 \
  -H "Authorization: Bearer ${api_key}" \
  "${probe_url}" >/dev/null; then
  printf "==> live preflight (ok: %s)\n" "${probe_url}"
  exit 0
fi

cat >&2 <<EOF
Live test preflight failed: no seeded soffio API responded at ${probe_url}.

Prepare the local live-test environment manually:
  docker compose -f docker-compose-dev.yml up -d
  SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev \\
    cargo run --bin soffio -- import seed/seed.toml
  SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev \\
    cargo run --bin soffio -- renderall
  SOFFIO__DATABASE__URL=postgres://soffio:soffio_local_dev@localhost:5432/soffio_dev \\
    target/debug/soffio serve

Or run the one-shot local wrapper:
  ./scripts/gate-full-local.sh

To skip live tests explicitly:
  SKIP_LIVE_TESTS=1 ./scripts/gate-full.sh
EOF
exit 1
