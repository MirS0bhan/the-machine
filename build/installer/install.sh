#!/usr/bin/env bash
# Live installer for The Machine OS (G13 bare-metal).
set -euo pipefail

TARGET_DISK="${1:-}"
ROOTFS_SRC="${2:-/workspace/build/rootfs}"

if [[ -z "${TARGET_DISK}" ]]; then
  echo "Usage: $0 <target-disk> [rootfs-path]" >&2
  echo "Example: sudo $0 /dev/sda" >&2
  exit 1
fi

if [[ ! -d "${ROOTFS_SRC}" ]]; then
  echo "ERROR: rootfs not found at ${ROOTFS_SRC}. Run: make rootfs-release" >&2
  exit 1
fi

if [[ ! -e "${ROOTFS_SRC}/vmlinuz" && ! -e "${ROOTFS_SRC}/boot/vmlinuz" ]]; then
  echo "ERROR: rootfs has no kernel (missing vmlinuz). Re-run build/mkrootfs.sh with debootstrap." >&2
  exit 1
fi

echo "==> The Machine installer"
echo "    Target: ${TARGET_DISK}"
echo "    Source: ${ROOTFS_SRC}"
read -r -p "This will ERASE ${TARGET_DISK}. Continue? [y/N] " confirm
if [[ "${confirm}" != "y" && "${confirm}" != "Y" ]]; then
  echo "Aborted."
  exit 0
fi

parted -s "${TARGET_DISK}" mklabel gpt
parted -s "${TARGET_DISK}" mkpart primary ext4 1MiB 100%
partprobe "${TARGET_DISK}" 2>/dev/null || true
sleep 2
PART="${TARGET_DISK}1"
[[ "${TARGET_DISK}" == *"nvme"* ]] && PART="${TARGET_DISK}p1"

mkfs.ext4 -F -L the-machine "${PART}"
MNT=$(mktemp -d)
mount "${PART}" "${MNT}"
rsync -a "${ROOTFS_SRC}/" "${MNT}/"
mkdir -p "${MNT}/boot/grub"

# Prefer rootfs-bundled kernel; fall back to host kernel for skeleton installs.
if [[ ! -f "${MNT}/boot/vmlinuz" ]]; then
  KERNEL="$(ls -1 /boot/vmlinuz-* 2>/dev/null | sort -V | tail -1 || true)"
  if [[ -n "${KERNEL}" ]]; then
    cp "${KERNEL}" "${MNT}/boot/vmlinuz"
  fi
fi
if [[ ! -f "${MNT}/boot/initrd.img" ]]; then
  INITRD="$(ls -1 /boot/initrd.img-* 2>/dev/null | sort -V | tail -1 || true)"
  if [[ -n "${INITRD}" ]]; then
    cp "${INITRD}" "${MNT}/boot/initrd.img"
  fi
fi

grub-install --target=i386-pc --boot-directory="${MNT}/boot" "${TARGET_DISK}" 2>/dev/null || \
  grub-install --boot-directory="${MNT}/boot" "${TARGET_DISK}" 2>/dev/null || \
  echo "WARN: grub-install failed — install GRUB manually" >&2

if [[ -f "${MNT}/boot/initrd.img" ]]; then
  cat > "${MNT}/boot/grub/grub.cfg" <<'GRUB'
set timeout=3
menuentry "The Machine" {
  linux /boot/vmlinuz root=LABEL=the-machine rw quiet
  initrd /boot/initrd.img
}
GRUB
else
  cat > "${MNT}/boot/grub/grub.cfg" <<'GRUB'
set timeout=3
menuentry "The Machine" {
  linux /boot/vmlinuz root=LABEL=the-machine rw quiet
}
GRUB
fi

sync
umount "${MNT}"
rmdir "${MNT}"
echo "==> Installation complete on ${TARGET_DISK} (root=LABEL=the-machine)"
