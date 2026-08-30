#!/usr/bin/env bash
# Regression: initramfs ships boot logging scripts and init uses them.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE="${ROOT}/build/initramfs.stage"

bash "${ROOT}/build/mkinitramfs.sh" debug >/dev/null

for f in init boot-log-lib.sh collect-boot-logs.sh; do
  [[ -f "${STAGE}/${f}" ]] || {
    echo "FAIL: ${STAGE}/${f} missing" >&2
    exit 1
  }
done

grep -q 'boot-log-lib.sh' "${STAGE}/init" || {
  echo "FAIL: init does not source boot-log-lib.sh" >&2
  exit 1
}

grep -q 'boot_log' "${STAGE}/init" || {
  echo "FAIL: init does not call boot_log" >&2
  exit 1
}

grep -q 'the-machine.debug' "${ROOT}/build/iso/boot/grub/grub.cfg" 2>/dev/null || {
  # iso dir may not exist until mkiso runs; validate mkiso.sh source instead
  grep -q 'the-machine.debug' "${ROOT}/build/mkiso.sh" || {
    echo "FAIL: debug GRUB entry missing from mkiso.sh" >&2
    exit 1
  }
}

echo "==> boot logging initramfs layout ok"
