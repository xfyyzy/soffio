#!/bin/sh
# Common mirror helpers shared across deployment targets.
#
# Functions here should be idempotent and safe to source multiple times.
set -eu

log() {
  printf '[mirror] %s\n' "$*" >&2
}

detect_country() {
  tz_lower=$(printf '%s' "${TZ:-}" | tr '[:upper:]' '[:lower:]')
  case "${tz_lower}" in
    asia/harbin|asia/beijing|asia/urumqi|asia/kashgar|asia/shanghai|asia/chongqing)
      COUNTRY="CN"
      return 0
      ;;
  esac

  if ! command -v curl >/dev/null 2>&1; then
    return 0
  fi

  for entry in \
    "https://api.ipapi.is .location.country_code" \
    "https://ifconfig.co/json .country_iso" \
    "https://api.ip2location.io .country_code" \
    "https://ipinfo.io/json .country" \
    "https://api.ipquery.io/?format=json .location.country_code" \
    "https://api.myip.com .cc"; do
    url=$(printf '%s' "${entry}" | awk '{print $1}')
    selector=$(printf '%s' "${entry}" | awk '{print $2}')
    if mirror_get_country "${url}" "${selector}"; then
      break
    fi
  done
}

mirror_get_country() {
  url="$1"
  selector="$2"

  if ! response=$(curl -fsSL --connect-timeout 3 --max-time 5 "$url" 2>/dev/null); then
    return 1
  fi

  code=""

  if command -v python3 >/dev/null 2>&1; then
    code=$(python3 - <<PY || true
import json, sys
from functools import reduce

raw = sys.stdin.read()
try:
    data = json.loads(raw)
except json.JSONDecodeError:
    sys.exit(0)

key_path = "${selector}".lstrip(".")
if not key_path:
    sys.exit(0)

def lookup(obj, key):
    if isinstance(obj, dict):
        return obj.get(key)
    return None

value = reduce(lookup, key_path.split("."), data)
if isinstance(value, str):
    print(value.strip())
PY
    )
  fi

  if [ -z "${code}" ]; then
    key=$(printf '%s' "${selector}" | awk -F '.' '{print $NF}')
    code=$(printf '%s' "${response}" | tr -d '\n' | sed -n "s/.*\"${key}\"[[:space:]]*:[[:space:]]*\"\\([A-Z][A-Z]\\)\".*/\\1/p" | head -n1)
  fi

  if [ -z "${code}" ]; then
    return 1
  fi

  COUNTRY="${code}"
  return 0
}

configure_debian_mirror() {
  log "configuring APT mirror for CN"
  if [ -r /etc/os-release ]; then
    # shellcheck source=/dev/null
    . /etc/os-release
  fi
  codename="${VERSION_CODENAME:-bookworm}"

  rm -f /etc/apt/sources.list.d/debian.sources

  cat >/etc/apt/sources.list <<EOF
deb https://mirrors.ustc.edu.cn/debian/ ${codename} main contrib non-free non-free-firmware
deb https://mirrors.ustc.edu.cn/debian/ ${codename}-updates main contrib non-free non-free-firmware
deb https://mirrors.ustc.edu.cn/debian/ ${codename}-backports main contrib non-free non-free-firmware
# Security repository intentionally disabled; restore the upstream source if needed:
# deb http://deb.debian.org/debian-security ${codename}-security main contrib non-free non-free-firmware
EOF
}

configure_alpine_mirror() {
  log "configuring APK mirror for CN"

  if [ ! -r /etc/apk/repositories ]; then
    log "apk repositories file missing; skipping"
    return 0
  fi

  release=""
  if [ -r /etc/os-release ]; then
    # shellcheck source=/dev/null
    . /etc/os-release
    if [ -n "${VERSION_ID:-}" ]; then
      release=$(printf '%s' "${VERSION_ID}" | awk -F '.' '{printf "%s.%s", $1, $2}' 2>/dev/null)
    fi
  fi

  if [ -z "${release}" ] && [ -r /etc/alpine-release ]; then
    release=$(awk -F '.' '{printf "%s.%s", $1, $2}' /etc/alpine-release 2>/dev/null)
  fi

  if [ -z "${release}" ]; then
    release_path="latest-stable"
  else
    release_path="v${release}"
  fi

  cat >/etc/apk/repositories <<EOF
https://mirrors.ustc.edu.cn/alpine/${release_path}/main
https://mirrors.ustc.edu.cn/alpine/${release_path}/community
EOF
}

configure_pkg_mirror() {
  log "configuring FreeBSD pkg mirror for CN"
  mkdir -p /usr/local/etc/pkg/repos
  cat >/usr/local/etc/pkg/repos/FreeBSD.conf <<'EOF'
FreeBSD: {
  url: "pkg+https://mirrors.ustc.edu.cn/freebsd-pkg/${ABI}/latest",
  mirror_type: "srv",
  signature_type: "fingerprints",
  fingerprints: "/usr/share/keys/pkg",
  enabled: yes
}
EOF
}

configure_cargo_mirror() {
  log "configuring Cargo mirror for CN"
  if [ -n "${CARGO_HOME:-}" ]; then
    cargo_homes="${CARGO_HOME}"
  else
    cargo_homes="/usr/local/cargo /root/.cargo"
  fi

  for cargo_home in ${cargo_homes}; do
    mkdir -p "${cargo_home}"
    cat >"${cargo_home}/config.toml" <<'EOF'
[source.crates-io]
replace-with = "tuna-sparse"

[source.tuna-sparse]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"

[source.tuna]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
EOF
  done
}

configure_node_mirror() {
  log "configuring Node.js and npm mirrors for CN"
  config_dir="/etc/soffio"
  node_mirror="https://npmmirror.com/mirrors/node"
  npm_registry="https://registry.npmmirror.com"

  mkdir -p "${config_dir}"

  printf '%s\n' "${node_mirror}" > "${config_dir}/node-mirror"
  printf '%s\n' "${npm_registry}" > "${config_dir}/npm-registry"
  printf 'registry=%s\n' "${npm_registry}" > /root/.npmrc
}

configure_puppeteer_mirror() {
  log "configuring Puppeteer mirror for CN"
  config_dir="/etc/soffio"
  puppeteer_base="https://cdn.npmmirror.com/binaries/chrome-for-testing"

  mkdir -p "${config_dir}"

  printf '%s\n' "${puppeteer_base}" > "${config_dir}/puppeteer-download-base"
}
