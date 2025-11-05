#!/bin/sh
set -eu

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=../mirrors/common.sh
. "${SCRIPT_DIR}/../mirrors/common.sh"

COUNTRY="${COUNTRY:-}"
detect_country

if [ "${COUNTRY:-}" != "CN" ]; then
  log "country detected as ${COUNTRY:-unknown}, skipping mirror configuration"
  exit 0
fi

if command -v apk >/dev/null 2>&1; then
  configure_alpine_mirror
elif command -v apt-get >/dev/null 2>&1; then
  configure_debian_mirror
else
  log "no supported package manager detected; skipping system mirror setup"
fi

configure_cargo_mirror
configure_node_mirror
configure_puppeteer_mirror
