#!/usr/bin/env bash
# Validate a The Machine rootfs tree (G13 — bare-metal install readiness).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=rootfs-common.sh
source "${SCRIPT_DIR}/rootfs-common.sh"

ROOTFS="${1:-}"
if [[ -z "${ROOTFS}" || ! -d "${ROOTFS}" ]]; then
  echo "Usage: $0 <rootfs-path>" >&2
  exit 1
fi

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

ok() {
  echo "OK: $*"
}

SERVICES=("${ROOTFS_SERVICES[@]}")

for svc in "${SERVICES[@]}"; do
  [[ -x "${ROOTFS}/the-machine/${svc}" ]] || fail "missing executable /the-machine/${svc}"
done
ok "all ${#SERVICES[@]} service binaries present"

[[ -f "${ROOTFS}/etc/the-machine/machine.conf" ]] || fail "missing /etc/the-machine/machine.conf"
grep -q 'THE_MACHINE_SOCKET_DIR=/run/the-machine' "${ROOTFS}/etc/the-machine/machine.conf" \
  || fail "machine.conf missing THE_MACHINE_SOCKET_DIR"
grep -q 'XDG_RUNTIME_DIR=/run/the-machine' "${ROOTFS}/etc/the-machine/machine.conf" \
  || fail "machine.conf missing XDG_RUNTIME_DIR"
grep -q 'THE_MACHINE_BOOT_AUIL=/etc/the-machine/boot.auil' "${ROOTFS}/etc/the-machine/machine.conf" \
  || fail "machine.conf missing THE_MACHINE_BOOT_AUIL"
ok "machine.conf runtime env"

BOOT_AUIL="${ROOTFS}/etc/the-machine/boot.auil"
[[ -f "${BOOT_AUIL}" ]] || fail "missing /etc/the-machine/boot.auil (boot greet layout)"
for widget in ui.greeting ui.chat_log ui.chat_input ui.chat_send; do
  grep -q "${widget}" "${BOOT_AUIL}" || fail "boot.auil missing widget ${widget}"
done
grep -q 'agent.chat.send' "${BOOT_AUIL}" || fail "boot.auil missing agent.chat.send binding"
ok "boot.auil greet + chat layout"

[[ -f "${ROOTFS}/usr/lib/systemd/system/the-machine.target" ]] || fail "missing the-machine.target"
if [[ "${THE_MACHINE_VALIDATE_INSTALLED:-0}" == "1" ]]; then
  for svc in "${SERVICES[@]}"; do
    [[ -f "${ROOTFS}/usr/lib/systemd/system/the-machine-${svc}.service" ]] \
      || fail "missing unit the-machine-${svc}.service"
  done
  ok "all ${#SERVICES[@]} systemd service units"
else
  for svc in system-daemon mcp-bus compositor; do
    [[ -f "${ROOTFS}/usr/lib/systemd/system/the-machine-${svc}.service" ]] \
      || fail "missing unit the-machine-${svc}.service"
  done
  ok "core systemd units"
fi

if [[ -e "${ROOTFS}/vmlinuz" || -e "${ROOTFS}/boot/vmlinuz" ]]; then
  ok "kernel symlink or /boot/vmlinuz present"
elif [[ "${THE_MACHINE_ROOTFS_VALIDATE_SKIP_KERNEL:-0}" == "1" ]]; then
  echo "WARN: no kernel in rootfs (THE_MACHINE_ROOTFS_VALIDATE_SKIP_KERNEL=1)" >&2
else
  fail "no kernel (expected /vmlinuz or /boot/vmlinuz — run mkrootfs with debootstrap)"
fi

if [[ -f "${ROOTFS}/etc/udev/rules.d/99-the-machine.rules" ]]; then
  ok "udev hotplug rules installed"
else
  echo "WARN: missing udev rules (optional for dev skeleton)" >&2
fi

if [[ -d "${ROOTFS}/usr/bin/apt-get" || -x "${ROOTFS}/usr/bin/apt-get" ]]; then
  ok "debootstrap rootfs detected"
fi

if [[ "${THE_MACHINE_VALIDATE_INSTALLED:-0}" == "1" ]]; then
  [[ -f "${ROOTFS}/etc/fstab" ]] || fail "missing /etc/fstab (installer must write fstab for target HW boot)"
  grep -qE '^LABEL=the-machine[[:space:]]+/[[:space:]]+ext4' "${ROOTFS}/etc/fstab" \
    || fail "fstab missing LABEL=the-machine root entry"
  ok "fstab root entry"
  bash "${SCRIPT_DIR}/verify-rootfs-grub.sh" "${ROOTFS}"
elif [[ -f "${ROOTFS}/boot/grub/grub.cfg" ]]; then
  bash "${SCRIPT_DIR}/verify-rootfs-grub.sh" "${ROOTFS}"
fi

echo "==> rootfs validation passed: ${ROOTFS}"
