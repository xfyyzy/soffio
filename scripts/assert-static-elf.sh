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

  if readelf -l "$binary" | grep -q 'INTERP'; then
    echo "static ELF check failed: $binary has a program interpreter" >&2
    readelf -l "$binary" | grep -n 'INTERP\|Requesting' >&2 || true
    exit 1
  fi

  if ! readelf -d "$binary" 2>&1 | grep -q 'There is no dynamic section'; then
    echo "static ELF check failed: $binary has a dynamic section" >&2
    readelf -d "$binary" >&2 || true
    exit 1
  fi
done
