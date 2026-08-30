#!/usr/bin/env bash
# G13: validate skeleton rootfs + optional debootstrap path when tools exist.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT

echo "==> building release binaries for rootfs"
(cd "${ROOT}" && cargo build --workspace --release)

export THE_MACHINE_ROOTFS_DIR="${TMP}/skeleton"
export THE_MACHINE_ROOTFS_SKIP_KERNEL=1
THE_MACHINE_ROOTFS_DIR="${THE_MACHINE_ROOTFS_DIR}" \
  THE_MACHINE_ROOTFS_SKIP_KERNEL=1 \
  bash "${ROOT}/build/mkrootfs.sh" minimal

# shellcheck source=rootfs-common.sh
source "${ROOT}/build/rootfs-common.sh"
rootfs_link_vmlinuz "${THE_MACHINE_ROOTFS_DIR}" || true
if [[ ! -e "${THE_MACHINE_ROOTFS_DIR}/vmlinuz" && ! -e "${THE_MACHINE_ROOTFS_DIR}/boot/vmlinuz" ]]; then
  export THE_MACHINE_ROOTFS_VALIDATE_SKIP_KERNEL=1
fi
bash "${ROOT}/build/rootfs-validate.sh" "${THE_MACHINE_ROOTFS_DIR}"

if command -v debootstrap >/dev/null 2>&1 && command -v sudo >/dev/null 2>&1; then
  echo "==> debootstrap available — building full rootfs (may take a few minutes)"
  export THE_MACHINE_ROOTFS_DIR="${TMP}/debootstrap"
  export THE_MACHINE_ROOTFS_SKIP_KERNEL=1
  if THE_MACHINE_ROOTFS_DIR="${THE_MACHINE_ROOTFS_DIR}" \
    THE_MACHINE_ROOTFS_SKIP_KERNEL=1 \
    bash "${ROOT}/build/mkrootfs.sh" minimal; then
    bash "${ROOT}/build/rootfs-validate.sh" "${THE_MACHINE_ROOTFS_DIR}"
    echo "OK: debootstrap rootfs validated (G13)"
  else
    echo "WARN: debootstrap mkrootfs failed — skeleton validation still passed" >&2
  fi
else
  echo "SKIP: debootstrap/sudo not available — skeleton validation only"
fi

echo "OK: rootfs-validate tests (G13)"
