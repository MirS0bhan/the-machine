#!/usr/bin/env bash
# Regression test: initramfs can resolve busybox via fetch-busybox fallback.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT

export PATH="${TMP}/empty:${PATH}"
mkdir -p "${TMP}/empty"
export THE_MACHINE_SKIP_BUSYBOX_FETCH=0
rm -rf "${ROOT}/build/tools/busybox"

path="$(bash "${ROOT}/build/fetch-busybox.sh" | tail -1)"
[[ -x "${path}" ]] || { echo "FAIL: fetch-busybox did not produce executable" >&2; exit 1; }
"${path}" sh -c 'echo ok' | grep -qx ok || { echo "FAIL: fetched busybox cannot run sh" >&2; exit 1; }

echo "OK: fetch-busybox resolves static busybox when not on PATH"
