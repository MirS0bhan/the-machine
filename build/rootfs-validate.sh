#!/usr/bin/env bash
# Validate a The Machine rootfs tree (G13 — bare-metal install readiness).
set -euo pipefail

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
ok "machine.conf runtime env"

[[ -f "${ROOTFS}/usr/lib/systemd/system/the-machine.target" ]] || fail "missing the-machine.target"
for svc in system-daemon mcp-bus compositor; do
  [[ -f "${ROOTFS}/usr/lib/systemd/system/the-machine-${svc}.service" ]] \
    || fail "missing unit the-machine-${svc}.service"
done
ok "systemd units"

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

echo "==> rootfs validation passed: ${ROOTFS}"
