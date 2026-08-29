#!/usr/bin/env bash
# Build a bootable ISO (GRUB + kernel + initramfs) for The Machine.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT}/build"
ISO_DIR="${BUILD_DIR}/iso"
KERNEL="${1:-}"
INITRAMFS="${2:-${BUILD_DIR}/initramfs.cpio.gz}"
OUTPUT="${3:-${BUILD_DIR}/the-machine.iso}"

if [[ -z "${KERNEL}" ]]; then
  KERNEL="$(ls -1 /boot/vmlinuz-* 2>/dev/null | sort -V | tail -1 || true)"
fi

if [[ -z "${KERNEL}" || ! -f "${KERNEL}" ]]; then
  echo "ERROR: No kernel found. Install linux-image-virtual or set KERNEL=/path/to/vmlinuz" >&2
  exit 1
fi

if [[ ! -f "${INITRAMFS}" ]]; then
  echo "ERROR: initramfs not found at ${INITRAMFS}. Run 'make initramfs' first." >&2
  exit 1
fi

echo "==> Building ISO"
rm -rf "${ISO_DIR}"
mkdir -p "${ISO_DIR}/boot/grub"

if [[ -n "${KERNEL}" && -f "${KERNEL}" ]]; then
  cp "${KERNEL}" "${ISO_DIR}/boot/vmlinuz" 2>/dev/null || sudo cp "${KERNEL}" "${ISO_DIR}/boot/vmlinuz"
else
  echo "ERROR: kernel required for ISO" >&2
  exit 1
fi
cp "${INITRAMFS}" "${ISO_DIR}/boot/initramfs.cpio.gz"

cat > "${ISO_DIR}/boot/grub/grub.cfg" <<'GRUB'
set timeout=3
set default=0

menuentry "The Machine" {
  linux /boot/vmlinuz console=ttyS0,115200 rdinit=/init quiet
  initrd /boot/initramfs.cpio.gz
}

menuentry "The Machine (debug shell)" {
  linux /boot/vmlinuz console=ttyS0,115200 rdinit=/init single
  initrd /boot/initramfs.cpio.gz
}
GRUB

mkdir -p "${BUILD_DIR}"
if command -v grub-mkrescue >/dev/null 2>&1; then
  grub-mkrescue -o "${OUTPUT}" "${ISO_DIR}" 2>/dev/null \
    || xorriso -as mkisofs -R -J -o "${OUTPUT}" "${ISO_DIR}"
else
  xorriso -as mkisofs -R -J -o "${OUTPUT}" "${ISO_DIR}"
fi

echo "==> ISO written to ${OUTPUT} ($(du -h "${OUTPUT}" | cut -f1))"
