#!/usr/bin/env bash
# Validate GRUB layout for a The Machine installed rootfs (G13).
set -euo pipefail

ROOTFS="${1:?rootfs path required}"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

ok() {
  echo "OK: $*"
}

GRUB_CFG="${ROOTFS}/boot/grub/grub.cfg"
[[ -f "${GRUB_CFG}" ]] || fail "missing ${GRUB_CFG}"

grep -q 'root=LABEL=the-machine' "${GRUB_CFG}" \
  || fail "grub.cfg missing root=LABEL=the-machine"
grep -qE 'linux /boot/vmlinuz|linux /vmlinuz' "${GRUB_CFG}" \
  || fail "grub.cfg missing linux /boot/vmlinuz entry"
ok "grub.cfg boot entry"

if [[ -f "${ROOTFS}/boot/vmlinuz" || -L "${ROOTFS}/boot/vmlinuz" ]]; then
  ok "/boot/vmlinuz present"
else
  fail "missing /boot/vmlinuz"
fi

if grep -q 'initrd /boot/initrd.img' "${GRUB_CFG}"; then
  [[ -f "${ROOTFS}/boot/initrd.img" ]] || fail "grub.cfg references initrd but file missing"
  ok "initrd referenced and present"
fi

FSTAB="${ROOTFS}/etc/fstab"
[[ -f "${FSTAB}" ]] || fail "missing ${FSTAB} (installer must write fstab for target HW boot)"
grep -qE '^LABEL=the-machine[[:space:]]+/[[:space:]]+ext4' "${FSTAB}" \
  || fail "fstab missing LABEL=the-machine root entry"
ok "fstab root entry"

echo "==> GRUB validation passed: ${ROOTFS}"
