#!/usr/bin/env sh
set -eu

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <elf>..." >&2
  exit 2
fi

for binary in "$@"; do
  if [ ! -f "$binary" ]; then
    echo "static ELF check failed: $binary does not exist" >&2
    exit 1
  fi

  file "$binary"

  if ! readelf -h "$binary" >/dev/null 2>&1; then
    echo "static ELF check failed: $binary is not a readable ELF binary" >&2
    file "$binary" >&2 || true
    exit 1
  fi

  if readelf -l "$binary" | grep -q 'INTERP'; then
    echo "static ELF check failed: $binary has a program interpreter" >&2
    readelf -l "$binary" | grep -n 'INTERP\|Requesting' >&2 || true
    exit 1
  fi

  if readelf -d "$binary" 2>/dev/null | grep -q '(NEEDED)'; then
    echo "static ELF check failed: $binary declares shared library dependencies" >&2
    readelf -d "$binary" | grep '(NEEDED)' >&2 || true
    exit 1
  fi
done
