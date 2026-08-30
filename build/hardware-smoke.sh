#!/usr/bin/env bash
# QEMU / CI hardware smoke checks for bare-metal readiness (G13).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FAIL=0

check() {
  if "$@"; then
    echo "OK: $*"
  else
    echo "FAIL: $*" >&2
    FAIL=1
  fi
}

echo "==> The Machine hardware smoke checks"

# Build + unit tests
check bash -c "cd '${ROOT}' && cargo test -p compositor -p system-daemon --quiet"
check bash -c "cd '${ROOT}' && make verify-docs >/dev/null"

# Rootfs layout (skeleton path)
check bash "${ROOT}/build/test-rootfs-validate.sh"

# DRM sysfs helper (no hardware required)
check bash -c "cd '${ROOT}' && cargo test -p common drm_sysfs --quiet"

if [[ -c /dev/dri/card0 ]]; then
  echo "OK: /dev/dri/card0 present"
else
  echo "SKIP: no DRM device (expected in CI VM)"
fi

if [[ "${FAIL}" -ne 0 ]]; then
  echo "==> hardware smoke FAILED" >&2
  exit 1
fi

echo "==> hardware smoke passed"
