#!/usr/bin/env bash
# G13: loopback install + GRUB validation (no physical disk required).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'sudo umount "${TMP}/mnt" 2>/dev/null || true; sudo losetup -d "${LOOP:-}" 2>/dev/null || true; rm -rf "${TMP}"' EXIT

if ! command -v sudo >/dev/null 2>&1; then
  echo "SKIP: sudo required for loopback installer test"
  exit 0
fi

for cmd in losetup parted mkfs.ext4 mount rsync; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "SKIP: ${cmd} not available for loopback installer test"
    exit 0
  fi
done

echo "==> building skeleton rootfs for installer test"
(cd "${ROOT}" && cargo build --workspace --release >/dev/null)

export THE_MACHINE_ROOTFS_DIR="${TMP}/rootfs"
export THE_MACHINE_ROOTFS_SKIP_KERNEL=1
THE_MACHINE_ROOTFS_DIR="${THE_MACHINE_ROOTFS_DIR}" \
  THE_MACHINE_ROOTFS_SKIP_KERNEL=1 \
  bash "${ROOT}/build/mkrootfs.sh" minimal

# shellcheck source=rootfs-common.sh
source "${ROOT}/build/rootfs-common.sh"
rootfs_link_vmlinuz "${THE_MACHINE_ROOTFS_DIR}" || true
if [[ ! -e "${THE_MACHINE_ROOTFS_DIR}/boot/vmlinuz" ]]; then
  echo "fake-kernel" | sudo tee "${THE_MACHINE_ROOTFS_DIR}/boot/vmlinuz" >/dev/null
fi
echo "fake-initrd" | sudo tee "${THE_MACHINE_ROOTFS_DIR}/boot/initrd.img" >/dev/null

IMG="${TMP}/disk.img"
truncate -s 256M "${IMG}"

echo "==> running installer on loopback disk"
LOOP="$(sudo losetup -f --show -P "${IMG}")"
THE_MACHINE_INSTALLER_YES=1 \
  sudo env THE_MACHINE_INSTALLER_YES=1 bash "${ROOT}/build/installer/install.sh" "${LOOP}" "${THE_MACHINE_ROOTFS_DIR}"

PART="${LOOP}p1"
MNT="${TMP}/mnt"
sudo mkdir -p "${MNT}"
sudo mount "${PART}" "${MNT}"

export THE_MACHINE_ROOTFS_VALIDATE_SKIP_KERNEL=1
bash "${ROOT}/build/validate-installed-rootfs.sh" "${MNT}"

sudo umount "${MNT}"
echo "OK: loopback installer GRUB validation (G13)"
