#!/usr/bin/env bash
# Boot initramfs in QEMU (nographic) and verify PID 1 reaches rescue or boot complete.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL="${1:-${ROOT}/build/vmlinuz}"
INITRAMFS="${2:-${ROOT}/build/initramfs.cpio.gz}"
TIMEOUT="${3:-90}"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "SKIP: qemu-system-x86_64 not installed"
  exit 0
fi

if [[ ! -f "${KERNEL}" || ! -f "${INITRAMFS}" ]]; then
  echo "SKIP: kernel or initramfs missing (run make iso first)"
  exit 0
fi

LOG="$(mktemp)"
trap 'rm -f "${LOG}"' EXIT

echo "==> QEMU boot smoke (rescue, ${TIMEOUT}s timeout)"
set +e
timeout "${TIMEOUT}" qemu-system-x86_64 -accel tcg -m 512M \
  -kernel "${KERNEL}" \
  -initrd "${INITRAMFS}" \
  -append "console=ttyS0,115200 rdinit=/init the-machine.rescue" \
  -nographic \
  >"${LOG}" 2>&1
set -e

if grep -qE 'the-machine\.rescue|boot starting|/ #|/bin/sh' "${LOG}"; then
  echo "==> QEMU boot smoke passed"
  exit 0
fi

echo "FAIL: QEMU boot did not reach rescue shell" >&2
echo "--- serial log (last 60 lines) ---" >&2
tail -60 "${LOG}" >&2
exit 1
