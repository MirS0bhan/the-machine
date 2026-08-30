#!/usr/bin/env bash
# Build a Debian-style rootfs for bare-metal install (G13).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT}/build"
ROOTFS="${BUILD_DIR}/rootfs"
PROFILE="${1:-minimal}"

echo "==> Building rootfs (${PROFILE})"

DEBOOTSTRAP_OK=0
if command -v debootstrap >/dev/null 2>&1 && command -v sudo >/dev/null 2>&1; then
  rm -rf "${ROOTFS}"
  if sudo debootstrap --variant=minbase bookworm "${ROOTFS}" http://deb.debian.org/debian; then
    DEBOOTSTRAP_OK=1
    echo "==> Installing bare-metal packages in chroot"
    sudo chroot "${ROOTFS}" apt-get update -qq
    sudo chroot "${ROOTFS}" DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
      linux-image-amd64 \
      grub-pc \
      systemd \
      iproute2 \
      kmod \
      firmware-linux \
      mesa-utils \
      libgl1 \
      seatd \
      wpa-supplicant \
      pipewire \
      pipewire-pulse \
      ca-certificates \
      >/dev/null 2>&1 || echo "WARN: some packages failed to install" >&2
  else
    echo "WARN: debootstrap failed; using skeleton rootfs" >&2
  fi
else
  echo "WARN: debootstrap/sudo not available — skeleton rootfs only" >&2
fi

if [[ "${DEBOOTSTRAP_OK}" -eq 0 ]]; then
  rm -rf "${ROOTFS}"
  mkdir -p "${ROOTFS}"/{bin,sbin,etc,the-machine,boot,usr/lib/systemd/system,var/lib/the-machine,run/the-machine}
fi

# Install Machine service binaries.
BIN_DIR="${ROOT}/target/release"
if [[ ! -d "${BIN_DIR}" ]] || [[ ! -x "${BIN_DIR}/mcp-bus" ]]; then
  (cd "${ROOT}" && cargo build --workspace --release)
fi

SERVICES=(
  system-daemon mcp-bus policy-broker state-store event-bus
  lambda-server agent-core ui-runtime compositor fallback-shell
  local-model-daemon marketplace
)

mkdir -p "${ROOTFS}/the-machine" "${ROOTFS}/etc/the-machine" "${ROOTFS}/var/lib/the-machine" "${ROOTFS}/boot"
for svc in "${SERVICES[@]}"; do
  if [[ -x "${BIN_DIR}/${svc}" ]]; then
    install -m 0755 "${BIN_DIR}/${svc}" "${ROOTFS}/the-machine/${svc}"
  fi
done

# Copy host kernel + initrd when building a release rootfs (installer expects /boot).
if [[ "${PROFILE}" == "release" && "${DEBOOTSTRAP_OK}" -eq 1 ]]; then
  KERNEL="$(ls -1 /boot/vmlinuz-* 2>/dev/null | sort -V | tail -1 || true)"
  INITRD="$(ls -1 /boot/initrd.img-* 2>/dev/null | sort -V | tail -1 || true)"
  if [[ -n "${KERNEL}" ]]; then
    cp "${KERNEL}" "${ROOTFS}/boot/vmlinuz"
    echo "==> Copied kernel to rootfs/boot/vmlinuz"
  fi
  if [[ -n "${INITRD}" ]]; then
    cp "${INITRD}" "${ROOTFS}/boot/initrd.img"
    echo "==> Copied initrd to rootfs/boot/initrd.img"
  fi
fi

# systemd units
UNIT_DIR="${ROOTFS}/usr/lib/systemd/system"
mkdir -p "${UNIT_DIR}"
cat > "${UNIT_DIR}/the-machine.target" <<'UNIT'
[Unit]
Description=The Machine Agent-Native OS Stack
After=network-online.target systemd-udev-settle.service
Wants=network-online.target

[Install]
WantedBy=multi-user.target
UNIT

for svc in system-daemon mcp-bus policy-broker state-store event-bus lambda-server agent-core compositor ui-runtime fallback-shell local-model-daemon marketplace; do
  AFTER="the-machine-mcp-bus.service"
  [[ "${svc}" == "system-daemon" ]] && AFTER="network.target"
  [[ "${svc}" == "mcp-bus" ]] && AFTER="the-machine-system-daemon.service"
  cat > "${UNIT_DIR}/the-machine-${svc}.service" <<UNIT
[Unit]
Description=The Machine ${svc}
After=${AFTER}
PartOf=the-machine.target

[Service]
Type=simple
Environment=THE_MACHINE_SOCKET_DIR=/run/the-machine
Environment=XDG_RUNTIME_DIR=/run/the-machine
Environment=RUST_LOG=info
Environment=WAYLAND_DISPLAY=wayland-0
Environment=THE_MACHINE_COMPOSITOR_BACKEND=auto
Environment=STATE_STORE_BACKEND=sled
Environment=STATE_STORE_PATH=/var/lib/the-machine/state
ExecStart=/the-machine/${svc}
Restart=on-failure
RuntimeDirectory=the-machine

[Install]
WantedBy=the-machine.target
UNIT
done

cat > "${ROOTFS}/etc/the-machine/machine.conf" <<'CONF'
# The Machine OS configuration (installed system)
THE_MACHINE_SOCKET_DIR=/run/the-machine
XDG_RUNTIME_DIR=/run/the-machine
STATE_STORE_BACKEND=sled
STATE_STORE_PATH=/var/lib/the-machine/state
THE_MACHINE_LAMBDA_DIR=/var/lib/the-machine/lambdas
THE_MACHINE_COMPOSITOR_BACKEND=auto
LOCAL_MODEL_PATH=/var/lib/the-machine/models/machine-tiny.gguf
CONF

if [[ "${DEBOOTSTRAP_OK}" -eq 1 ]]; then
  sudo ln -sf /the-machine/mcp-bus "${ROOTFS}/etc/systemd/system/multi-user.target.wants/the-machine.target" 2>/dev/null || true
fi

echo "==> Rootfs written to ${ROOTFS} (debootstrap=${DEBOOTSTRAP_OK})"
