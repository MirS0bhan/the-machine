#!/usr/bin/env bash
# G13: validate installer GRUB template (root=LABEL=the-machine, vmlinuz path).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT

# shellcheck source=grub-installer.sh
source "${ROOT}/build/grub-installer.sh"

MNT="${TMP}/mnt"
mkdir -p "${MNT}/boot"
touch "${MNT}/boot/vmlinuz"

write_grub_cfg "${MNT}" 0
validate_grub_cfg "${MNT}/boot/grub/grub.cfg" || {
  echo "FAIL: grub.cfg without initrd" >&2
  exit 1
}
grep -q 'initrd /boot/initrd.img' "${MNT}/boot/grub/grub.cfg" && {
  echo "FAIL: unexpected initrd line in no-initrd cfg" >&2
  exit 1
}

write_grub_cfg "${MNT}" 1
validate_grub_cfg "${MNT}/boot/grub/grub.cfg" || {
  echo "FAIL: grub.cfg with initrd" >&2
  exit 1
}
grep -q 'initrd /boot/initrd.img' "${MNT}/boot/grub/grub.cfg" || {
  echo "FAIL: missing initrd line in initrd cfg" >&2
  exit 1
}

echo "OK: installer GRUB template (G13)"
