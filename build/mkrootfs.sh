#!/usr/bin/env bash
# Build a minimal Debian-style rootfs for The Machine (Phase 6).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT}/build"
ROOTFS="${BUILD_DIR}/rootfs"
PROFILE="${1:-minimal}"

echo "==> Building rootfs (${PROFILE})"

if ! command -v debootstrap >/dev/null 2>&1; then
  echo "WARN: debootstrap not found — creating skeleton rootfs only" >&2
  rm -rf "${ROOTFS}"
  mkdir -p "${ROOTFS}"/{bin,sbin,etc,the-machine,var/lib/the-machine,usr/lib/systemd/system}
else
  rm -rf "${ROOTFS}"
  sudo debootstrap --variant=minbase bookworm "${ROOTFS}" http://deb.debian.org/debian || {
    echo "debootstrap failed; using skeleton" >&2
    mkdir -p "${ROOTFS}"/{bin,sbin,etc,the-machine}
  }
fi

# Install Machine service binaries.
BIN_DIR="${ROOT}/target/release"
if [[ ! -d "${BIN_DIR}" ]]; then
  (cd "${ROOT}" && cargo build --workspace --release)
fi

SERVICES=(
  system-daemon mcp-bus policy-broker state-store event-bus
  lambda-server agent-core ui-runtime compositor fallback-shell
  local-model-daemon marketplace
)

mkdir -p "${ROOTFS}/the-machine" "${ROOTFS}/etc/the-machine" "${ROOTFS}/var/lib/the-machine"
for svc in "${SERVICES[@]}"; do
  if [[ -x "${BIN_DIR}/${svc}" ]]; then
    install -m 0755 "${BIN_DIR}/${svc}" "${ROOTFS}/the-machine/${svc}"
  fi
done

# systemd units
UNIT_DIR="${ROOTFS}/usr/lib/systemd/system"
mkdir -p "${UNIT_DIR}"
cat > "${UNIT_DIR}/the-machine.target" <<'UNIT'
[Unit]
Description=The Machine Agent-Native OS Stack
After=network.target

[Install]
WantedBy=multi-user.target
UNIT

for svc in mcp-bus policy-broker state-store event-bus lambda-server agent-core compositor ui-runtime; do
  cat > "${UNIT_DIR}/the-machine-${svc}.service" <<UNIT
[Unit]
Description=The Machine ${svc}
After=the-machine-mcp-bus.service
PartOf=the-machine.target

[Service]
Type=simple
Environment=THE_MACHINE_SOCKET_DIR=/run/the-machine
Environment=RUST_LOG=info
Environment=WAYLAND_DISPLAY=wayland-0
ExecStart=/the-machine/${svc}
Restart=on-failure
RuntimeDirectory=the-machine

[Install]
WantedBy=the-machine.target
UNIT
done

cat > "${ROOTFS}/etc/the-machine/machine.conf" <<'CONF'
# The Machine OS configuration
THE_MACHINE_SOCKET_DIR=/run/the-machine
STATE_STORE_BACKEND=sled
STATE_STORE_PATH=/var/lib/the-machine/state
THE_MACHINE_LAMBDA_DIR=/var/lib/the-machine/lambdas
CONF

echo "==> Rootfs written to ${ROOTFS}"
