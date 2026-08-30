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
  KERNEL="$(bash "${ROOT}/build/select-kernel.sh" || true)"
fi

if [[ -z "${KERNEL}" ]]; then
  KERNEL="$(ls -1 /boot/vmlinuz-* 2>/dev/null | sort -V | tail -1 || true)"
fi

if [[ -z "${KERNEL}" || ! -f "${KERNEL}" ]]; then
  echo "ERROR: No kernel found. Install linux-image-generic and set KERNEL=/boot/vmlinuz-*-generic" >&2
  exit 1
fi

bash "${ROOT}/build/select-kernel.sh" --warn-if-cloud "${KERNEL}"
echo "==> Using kernel ${KERNEL}"

if [[ ! -f "${INITRAMFS}" ]]; then
  echo "ERROR: initramfs not found at ${INITRAMFS}. Run 'make initramfs-release' first." >&2
  exit 1
fi

echo "==> Building ISO"
rm -rf "${ISO_DIR}"
mkdir -p "${ISO_DIR}/boot/grub"

# Kernel may be root-owned (e.g. on GitHub Actions after apt install).
KERNEL_STAGE="${BUILD_DIR}/vmlinuz"
if cp "${KERNEL}" "${KERNEL_STAGE}" 2>/dev/null; then
  :
elif sudo cp "${KERNEL}" "${KERNEL_STAGE}" 2>/dev/null; then
  sudo chmod a+r "${KERNEL_STAGE}"
else
  echo "ERROR: cannot read kernel at ${KERNEL}" >&2
  exit 1
fi
cp "${KERNEL_STAGE}" "${ISO_DIR}/boot/vmlinuz"
cp "${INITRAMFS}" "${ISO_DIR}/boot/initramfs.cpio.gz"

# grub-mkrescue runs xorriso as the current user; root-only files break grafting.
chmod -R a+rX "${ISO_DIR}" 2>/dev/null || sudo chmod -R a+rX "${ISO_DIR}"

cat > "${ISO_DIR}/boot/grub/grub.cfg" <<'GRUB'
set timeout=3
set default=0

menuentry "The Machine" {
  linux /boot/vmlinuz console=tty0 console=ttyS0,115200 rdinit=/init quiet
  initrd /boot/initramfs.cpio.gz
}

menuentry "The Machine (debug)" {
  linux /boot/vmlinuz console=tty0 console=ttyS0,115200 rdinit=/init the-machine.debug loglevel=7
  initrd /boot/initramfs.cpio.gz
}

menuentry "The Machine (rescue shell)" {
  linux /boot/vmlinuz console=tty0 console=ttyS0,115200 rdinit=/init the-machine.rescue
  initrd /boot/initramfs.cpio.gz
}
GRUB

mkdir -p "${BUILD_DIR}"
rm -f "${OUTPUT}"

if ! command -v grub-mkrescue >/dev/null 2>&1; then
  echo "ERROR: grub-mkrescue not found. Install grub-pc-bin, grub-common, xorriso, and mtools." >&2
  exit 1
fi
if ! command -v mformat >/dev/null 2>&1; then
  echo "ERROR: mformat not found. Install mtools (required by grub-mkrescue)." >&2
  exit 1
fi

echo "==> Running grub-mkrescue"
if ! grub-mkrescue -o "${OUTPUT}" "${ISO_DIR}"; then
  echo "ERROR: grub-mkrescue failed; refusing to emit a non-bootable ISO." >&2
  exit 1
fi

bash "${ROOT}/build/verify-iso-bootable.sh" "${OUTPUT}"

echo "==> ISO written to ${OUTPUT} ($(du -h "${OUTPUT}" | cut -f1))"
