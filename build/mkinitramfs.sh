#!/usr/bin/env bash
# Assemble a bootable initramfs with busybox and The Machine services.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT}/build"
STAGE="${BUILD_DIR}/initramfs.stage"
OUTPUT="${BUILD_DIR}/initramfs.cpio.gz"
PROFILE="${1:-debug}"

# shellcheck source=rootfs-common.sh
source "${ROOT}/build/rootfs-common.sh"

echo "==> Building initramfs (${PROFILE})"

rm -rf "${STAGE}"
mkdir -p "${STAGE}"/{bin,sbin,etc,the-machine,run/the-machine,proc,sys,dev,tmp,var/log}

# Busybox provides /bin/sh and core utilities in the initramfs.
BUSYBOX="$(bash "${ROOT}/build/fetch-busybox.sh" | tail -1)"
if [[ -z "${BUSYBOX}" || ! -x "${BUSYBOX}" ]]; then
  echo "ERROR: busybox not found. Install busybox-static or allow fetch." >&2
  exit 1
fi
cp "${BUSYBOX}" "${STAGE}/bin/busybox"
for app in sh ls cat echo mkdir mount umount sleep grep sed awk; do
  ln -sf busybox "${STAGE}/bin/${app}"
done

# Build Rust services (release for iso, debug for fast iteration).
if [[ "${PROFILE}" == "release" ]]; then
  cargo build --workspace --release
  BIN_DIR="${ROOT}/target/release"
else
  cargo build --workspace
  BIN_DIR="${ROOT}/target/debug"
fi

SERVICES=("${ROOTFS_SERVICES[@]}")

for svc in "${SERVICES[@]}"; do
  if [[ -x "${BIN_DIR}/${svc}" ]]; then
    install -m 0755 "${BIN_DIR}/${svc}" "${STAGE}/the-machine/${svc}"
  else
    echo "WARN: ${svc} binary not found at ${BIN_DIR}/${svc}" >&2
  fi
done

# Bundle GGUF model when available (G11).
MODEL_SRC="${ROOT}/build/models/machine-tiny.gguf"
if [[ -f "${MODEL_SRC}" ]]; then
  mkdir -p "${STAGE}/models"
  echo "==> Bundling GGUF model ($(du -h "${MODEL_SRC}" | cut -f1))"
  cp "${MODEL_SRC}" "${STAGE}/models/machine-tiny.gguf"
  export LOCAL_MODEL_PATH="/models/machine-tiny.gguf"
fi

# Boot AUIL layout (G6) + secrets directory (G1).
mkdir -p "${STAGE}/etc/the-machine" "${STAGE}/run/the-machine/secrets"
if [[ -f "${ROOT}/build/boot.auil" ]]; then
  cp "${ROOT}/build/boot.auil" "${STAGE}/etc/the-machine/boot.auil"
fi

# Init script: start services in boot order (L0 → L3 → L1 → L4 → L5).
cat > "${STAGE}/init" <<'INIT'
#!/bin/sh
export PATH=/bin:/sbin:/the-machine
export RUST_LOG=info
export THE_MACHINE_SOCKET_DIR=/run/the-machine
export XDG_RUNTIME_DIR=/run/the-machine
export WAYLAND_DISPLAY=wayland-0
export STATE_STORE_BACKEND=sled
export STATE_STORE_PATH=/var/the-machine/state
export THE_MACHINE_LAMBDA_DIR=/var/the-machine/lambdas
export LOCAL_MODEL_PATH=/models/machine-tiny.gguf
export THE_MACHINE_BOOT_AUIL=/etc/the-machine/boot.auil
export THE_MACHINE_COMPOSITOR_BACKEND=auto
mkdir -p /var/the-machine/state /var/the-machine/lambdas /models /run/the-machine/secrets /etc/the-machine

mount -t proc proc /proc
mount -t sysfs sys /sys
mount -t devtmpfs dev /dev 2>/dev/null || true
mkdir -p /run/the-machine /var/log

echo "[init] The Machine boot sequence starting"

start_svc() {
  name="$1"
  if [ -x "/the-machine/$name" ]; then
    echo "[init] starting $name"
    "/the-machine/$name" >>"/var/log/$name.log" 2>&1 &
  fi
}

# L0
start_svc system-daemon
sleep 1

# L3 (bus before broker consumers)
start_svc mcp-bus
sleep 1

# L2
start_svc policy-broker
sleep 1

# L1
start_svc state-store
start_svc event-bus
start_svc lambda-server
start_svc local-model-daemon
start_svc marketplace
sleep 1

# L4
start_svc agent-core
sleep 1

# L5 — display session: compositor first, then UI, then shell
start_svc compositor
sleep 2
start_svc ui-runtime
sleep 2
start_svc fallback-shell

echo "[init] boot complete — compositor + ui-runtime active"
echo "[init] VGA: tty0  serial: ttyS0  chat UI loads after agent greet"

# Keep PID 1 alive; services run in background.
while true; do sleep 3600; done
INIT
chmod +x "${STAGE}/init"

# Pack cpio archive (use busybox cpio when host cpio is absent).
mkdir -p "${BUILD_DIR}"
if command -v cpio >/dev/null 2>&1; then
  CPIO=(cpio -o -H newc)
else
  CPIO=("${BUSYBOX}" cpio -o -H newc)
fi
(
  cd "${STAGE}"
  find . | "${CPIO[@]}" 2>/dev/null | gzip -9 > "${OUTPUT}"
)

echo "==> Initramfs written to ${OUTPUT} ($(du -h "${OUTPUT}" | cut -f1))"
