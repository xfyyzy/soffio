#!/usr/bin/env sh
set -eu

# Keep build pressure conservative to avoid long stalls around target listing/build locks.
NEXTEST_BUILD_JOBS="${NEXTEST_BUILD_JOBS:-1}"

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

run_nextest() {
  label="$1"
  shift

  printf "==> nextest %s\n" "${label}"
  cargo nextest run \
    --workspace \
    --build-jobs "${NEXTEST_BUILD_JOBS}" \
    --cargo-quiet \
    --no-tests warn \
    "$@"
}

has_top_level_rs_targets() {
  dir="$1"
  [ -d "${dir}" ] || return 1
  first_target="$(find "${dir}" -maxdepth 1 -type f -name "*.rs" -print -quit)"
  [ -n "${first_target}" ]
}

run_nextest --lib --lib "$@"

# `cargo nextest ... --bins` and `--bin <name>` can hang on this workspace
# during target enumeration (no rustc activity, no output). The current binary
# targets do not contain runnable tests, so skip this shard by default to keep
# the full matrix deterministic. Set NEXTEST_RUN_BINS=1 to force a bins run.
if [ "${NEXTEST_RUN_BINS:-0}" = "1" ]; then
  run_nextest --bins --bins "$@"
else
  printf "==> nextest --bins (skipped: known nextest bin-target stall; set NEXTEST_RUN_BINS=1 to force)\n"
fi

# Running each integration test target separately avoids long stalls seen with a monolithic `--tests` listing pass.
test_targets_file="$(mktemp)"
trap 'rm -f "${test_targets_file}"' EXIT HUP INT TERM
find tests -maxdepth 1 -type f -name "*.rs" -print | LC_ALL=C sort > "${test_targets_file}"

while IFS= read -r test_file; do
  test_name="$(basename "${test_file}" .rs)"
  run_nextest "--test ${test_name}" --test "${test_name}" "$@"
done < "${test_targets_file}"

if has_top_level_rs_targets examples; then
  run_nextest --examples --examples "$@"
fi

if has_top_level_rs_targets benches; then
  run_nextest --benches --benches "$@"
fi
