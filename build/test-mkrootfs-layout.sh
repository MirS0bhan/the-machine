#!/usr/bin/env bash
# Regression test: rootfs kernel symlinks and debootstrap detection (G13).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT

# shellcheck source=rootfs-common.sh
source "${ROOT}/build/rootfs-common.sh"

ROOTFS="${TMP}/rootfs"
rootfs_skeleton_dirs "${ROOTFS}"

if rootfs_has_debootstrap "${ROOTFS}"; then
  echo "FAIL: skeleton rootfs must not look debootstrapped" >&2
  exit 1
fi

mkdir -p "${ROOTFS}/usr/bin"
echo '#!/bin/sh' > "${ROOTFS}/usr/bin/apt-get"
chmod +x "${ROOTFS}/usr/bin/apt-get"
if ! rootfs_has_debootstrap "${ROOTFS}"; then
  echo "FAIL: debootstrap rootfs not detected" >&2
  exit 1
fi

ROOTFS2="${TMP}/rootfs2"
rootfs_skeleton_dirs "${ROOTFS2}"
echo "fake-kernel" > "${ROOTFS2}/boot/vmlinuz-6.1.0-test"
if ! rootfs_link_vmlinuz "${ROOTFS2}"; then
  echo "FAIL: expected vmlinuz link from boot image" >&2
  exit 1
fi
[[ -L "${ROOTFS2}/vmlinuz" ]] || { echo "FAIL: missing /vmlinuz symlink" >&2; exit 1; }
[[ -L "${ROOTFS2}/boot/vmlinuz" ]] || { echo "FAIL: missing /boot/vmlinuz symlink" >&2; exit 1; }

# Skeleton mkrootfs path (no debootstrap, skip chroot kernel apt).
export THE_MACHINE_ROOTFS_DIR="${TMP}/mkrootfs-out"
export THE_MACHINE_ROOTFS_SKIP_KERNEL=1
THE_MACHINE_ROOTFS_DIR="${THE_MACHINE_ROOTFS_DIR}" \
  THE_MACHINE_ROOTFS_SKIP_KERNEL=1 \
  bash "${ROOT}/build/mkrootfs.sh" minimal

for svc in mcp-bus compositor system-daemon; do
  [[ -x "${THE_MACHINE_ROOTFS_DIR}/the-machine/${svc}" ]] || {
    echo "FAIL: missing service binary ${svc} in skeleton rootfs" >&2
    exit 1
  }
done
[[ -f "${THE_MACHINE_ROOTFS_DIR}/etc/the-machine/machine.conf" ]] || {
  echo "FAIL: missing machine.conf" >&2
  exit 1
}
grep -q 'THE_MACHINE_BOOT_AUIL=/etc/the-machine/boot.auil' \
  "${THE_MACHINE_ROOTFS_DIR}/etc/the-machine/machine.conf" || {
  echo "FAIL: machine.conf missing THE_MACHINE_BOOT_AUIL" >&2
  exit 1
}
[[ -f "${THE_MACHINE_ROOTFS_DIR}/etc/the-machine/boot.auil" ]] || {
  echo "FAIL: missing boot.auil in skeleton rootfs" >&2
  exit 1
}

echo "OK: rootfs layout + kernel symlink helpers (G13)"
