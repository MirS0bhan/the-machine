#!/usr/bin/env bash
# Shared GRUB config for bare-metal installer (G13).
set -euo pipefail

# Write /boot/grub/grub.cfg under MNT with root=LABEL=the-machine.
# Usage: write_grub_cfg <mount-root> [has-initrd: 0|1]
write_grub_cfg() {
  local mnt="${1:?mount root required}"
  local has_initrd="${2:-0}"
  mkdir -p "${mnt}/boot/grub"

  if [[ "${has_initrd}" == "1" ]]; then
    cat > "${mnt}/boot/grub/grub.cfg" <<'GRUB'
set timeout=3
menuentry "The Machine" {
  linux /boot/vmlinuz root=LABEL=the-machine rw quiet
  initrd /boot/initrd.img
}
GRUB
  else
    cat > "${mnt}/boot/grub/grub.cfg" <<'GRUB'
set timeout=3
menuentry "The Machine" {
  linux /boot/vmlinuz root=LABEL=the-machine rw quiet
}
GRUB
  fi
}

# Validate an existing grub.cfg matches installer expectations.
validate_grub_cfg() {
  local cfg="${1:?grub.cfg path required}"
  [[ -f "${cfg}" ]] || return 1
  grep -q 'root=LABEL=the-machine' "${cfg}" || return 1
  grep -q '/boot/vmlinuz' "${cfg}" || return 1
  return 0
}
