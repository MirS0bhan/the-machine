#!/usr/bin/env bash
# Full QEMU boot verification: initramfs/ISO must start services (not ': not found').
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:-initramfs}"   # initramfs | iso | vga
TIMEOUT="${QEMU_BOOT_TIMEOUT:-120}"
LOG="$(mktemp)"
trap 'rm -f "${LOG}"' EXIT

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "SKIP: qemu-system-x86_64 not installed"
  exit 0
fi

KERNEL="${KERNEL:-${ROOT}/build/vmlinuz}"
INITRAMFS="${INITRAMFS:-${ROOT}/build/initramfs.cpio.gz}"
ISO="${ISO:-${ROOT}/build/the-machine.iso}"

echo "==> QEMU full boot verify (${MODE}, ${TIMEOUT}s)"

case "${MODE}" in
  initramfs)
    if [[ ! -f "${INITRAMFS}" ]]; then
      bash "${ROOT}/build/mkinitramfs.sh" release >/dev/null
    fi
    if [[ ! -f "${KERNEL}" ]]; then
      K="$(bash "${ROOT}/build/select-kernel.sh" 2>/dev/null || true)"
      if [[ -n "${K}" && -f "${K}" ]]; then
        cp "${K}" "${KERNEL}" 2>/dev/null || sudo cp "${K}" "${KERNEL}"
        sudo chmod a+r "${KERNEL}" 2>/dev/null || chmod a+r "${KERNEL}" 2>/dev/null || true
      fi
    fi
    [[ -f "${KERNEL}" && -f "${INITRAMFS}" ]] || {
      echo "FAIL: kernel or initramfs missing (run make iso-release first)" >&2
      exit 1
    }
  qemu_args=(
    qemu-system-x86_64 -accel tcg -m 1G
    -kernel "${KERNEL}"
    -initrd "${INITRAMFS}"
    -append "console=ttyS0,115200 rdinit=/init the-machine.debug loglevel=7"
    -nographic
  )
    ;;
  iso)
    if [[ ! -f "${ISO}" ]]; then
      make -C "${ROOT}" iso-release >/dev/null
    fi
    [[ -f "${ISO}" ]] || { echo "FAIL: ISO missing (run make iso-release)" >&2; exit 1; }
    # GRUB passes console=ttyS0 on the debug menu entry; default entry also logs to serial.
    qemu_args=(
      qemu-system-x86_64 -accel tcg -m 1G
      -cdrom "${ISO}" -boot d
      -nographic
    )
    ;;
  vga)
    if [[ ! -f "${INITRAMFS}" ]]; then
      bash "${ROOT}/build/mkinitramfs.sh" release >/dev/null
    fi
    if [[ ! -f "${KERNEL}" ]]; then
      K="$(bash "${ROOT}/build/select-kernel.sh" 2>/dev/null || true)"
      [[ -n "${K}" && -f "${K}" ]] && cp "${K}" "${KERNEL}" 2>/dev/null || sudo cp "${K}" "${KERNEL}"
      sudo chmod a+r "${KERNEL}" 2>/dev/null || true
    fi
    [[ -f "${KERNEL}" && -f "${INITRAMFS}" ]] || {
      echo "FAIL: kernel or initramfs missing" >&2
      exit 1
    }
    qemu_args=(
      qemu-system-x86_64 -accel tcg -m 1G
      -kernel "${KERNEL}"
      -initrd "${INITRAMFS}"
      -append "console=ttyS0,115200 rdinit=/init the-machine.debug loglevel=7"
      -vga none -device virtio-vga
      -display none -serial mon:stdio
    )
    ;;
  *)
    echo "usage: $0 [initramfs|iso|vga]" >&2
    exit 2
    ;;
esac

set +e
timeout "${TIMEOUT}" "${qemu_args[@]}" >"${LOG}" 2>&1
qemu_rc=$?
set -e

# QEMU may exit when timeout kills it — we care about serial output.
if ! grep -q 'boot complete' "${LOG}"; then
  echo "FAIL: boot did not reach 'boot complete'" >&2
  echo "--- serial log (last 80 lines) ---" >&2
  tail -80 "${LOG}" >&2
  exit 1
fi

if grep -qE '/the-machine/[^: ]+: not found' "${LOG}"; then
  echo "FAIL: dynamic linker or binary missing (': not found' in log)" >&2
  grep -E 'not found|dynamic linker' "${LOG}" | tail -20 >&2
  exit 1
fi

# Core stack must be running at boot_dump_status time.
for svc in mcp-bus compositor ui-runtime agent-core; do
  if ! grep -q "${svc}: running" "${LOG}"; then
    echo "FAIL: ${svc} not running at boot status dump" >&2
    grep -E "${svc}:|${svc} log" "${LOG}" | tail -15 >&2
    exit 1
  fi
done

if grep -q 'compositor-backend: (not written yet)' "${LOG}"; then
  echo "WARN: compositor-backend not written (compositor may have crashed early)" >&2
  grep -E 'compositor' "${LOG}" | tail -20 >&2
  exit 1
fi

if grep -q 'backend=memory' "${LOG}"; then
  if [[ "${MODE}" == "vga" ]]; then
    echo "FAIL: compositor still on memory backend with virtio-vga (display modules missing?)" >&2
    grep -E 'fb0|dri|module|compositor-backend' "${LOG}" | tail -25 >&2
    exit 1
  fi
  echo "==> compositor on memory backend (expected for nographic / no virtio-vga)"
else
  echo "==> compositor backend: $(grep -o 'backend=[^ ]*' "${LOG}" | tail -1 || echo unknown)"
fi

echo "==> QEMU full boot verify passed (${MODE})"
exit 0
