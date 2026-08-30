#!/usr/bin/env bash
# Regression: root-owned vmlinuz in the ISO tree must not break grub-mkrescue.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

KERNEL="$(ls -1 /boot/vmlinuz-* 2>/dev/null | sort -V | tail -1 || true)"
INITRAMFS="${ROOT}/build/initramfs.cpio.gz"
if [[ -z "${KERNEL}" || ! -f "${KERNEL}" ]]; then
  echo "SKIP: no host kernel for mkiso bootable test"
  exit 0
fi
if [[ ! -f "${INITRAMFS}" ]]; then
  echo "SKIP: run make initramfs first"
  exit 0
fi

ISO_OUT="${WORK}/the-machine.iso"
bash "${ROOT}/build/mkiso.sh" "${KERNEL}" "${INITRAMFS}" "${ISO_OUT}"
bash "${ROOT}/build/verify-iso-bootable.sh" "${ISO_OUT}"
echo "OK: mkiso produces bootable ISO"
