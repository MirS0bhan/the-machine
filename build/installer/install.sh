#!/usr/bin/env bash
# Live installer for The Machine OS (Phase 6).
set -euo pipefail

TARGET_DISK="${1:-}"
ROOTFS_SRC="${2:-/workspace/build/rootfs}"

if [[ -z "${TARGET_DISK}" ]]; then
  echo "Usage: $0 <target-disk> [rootfs-path]" >&2
  echo "Example: $0 /dev/sda" >&2
  exit 1
fi

if [[ ! -d "${ROOTFS_SRC}" ]]; then
  echo "ERROR: rootfs not found at ${ROOTFS_SRC}. Run build/mkrootfs.sh first." >&2
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
partprobe "${TARGET_DISK}" || true
sleep 2
PART="${TARGET_DISK}1"
mkfs.ext4 -F "${PART}"
MNT=$(mktemp -d)
mount "${PART}" "${MNT}"
rsync -a "${ROOTFS_SRC}/" "${MNT}/"
mkdir -p "${MNT}/boot/grub"
grub-install --target=i386-pc --boot-directory="${MNT}/boot" "${TARGET_DISK}" || true
cat > "${MNT}/boot/grub/grub.cfg" <<'GRUB'
set timeout=3
menuentry "The Machine" {
  linux /vmlinuz root=LABEL=the-machine rw
}
GRUB
umount "${MNT}"
rmdir "${MNT}"
echo "==> Installation complete on ${TARGET_DISK}"
