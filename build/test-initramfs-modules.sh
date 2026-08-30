#!/usr/bin/env bash
# Regression: initramfs bundles display kernel modules for QEMU virtio-vga.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE="${ROOT}/build/initramfs.stage"

bash "${ROOT}/build/mkinitramfs.sh" release >/dev/null

KVER="$(basename "$(bash "${ROOT}/build/select-kernel.sh" 2>/dev/null || echo "/boot/vmlinuz-$(uname -r)")" | sed 's/^vmlinuz-//')"

[[ -f "${STAGE}/lib/modules/${KVER}/kernel/drivers/gpu/drm/virtio/virtio-gpu.ko" ]] || {
  echo "FAIL: virtio-gpu.ko not bundled (KVER=${KVER})" >&2
  exit 1
}

[[ -f "${STAGE}/lib/modules/${KVER}/kernel/drivers/virtio/virtio_dma_buf.ko" ]] || {
  echo "FAIL: virtio_dma_buf.ko not bundled" >&2
  exit 1
}

echo "==> initramfs display modules ok"
