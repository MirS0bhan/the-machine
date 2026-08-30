#!/usr/bin/env bash
# Regression: initramfs must include the dynamic linker for Rust service binaries.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE="${ROOT}/build/initramfs.stage"
OUTPUT="${ROOT}/build/initramfs.cpio.gz"

bash "${ROOT}/build/mkinitramfs.sh" debug >/dev/null

[[ -x "${STAGE}/the-machine/compositor" ]] || {
  echo "FAIL: compositor binary missing from initramfs stage" >&2
  exit 1
}

if ldd "${STAGE}/the-machine/compositor" >/dev/null 2>&1; then
  [[ -f "${STAGE}/lib64/ld-linux-x86-64.so.2" || -f "${STAGE}/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2" ]] || {
    echo "FAIL: dynamic linker not bundled into initramfs" >&2
    exit 1
  }
  [[ -f "${STAGE}/lib/x86_64-linux-gnu/libc.so.6" ]] || {
    echo "FAIL: libc not bundled into initramfs" >&2
    exit 1
  }
fi

echo "==> initramfs shared libraries ok"
