#!/usr/bin/env bash
# Assemble a bootable initramfs with busybox and The Machine services.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT}/build"
STAGE="${BUILD_DIR}/initramfs.stage"
OUTPUT="${BUILD_DIR}/initramfs.cpio.gz"
PROFILE="${1:-debug}"

echo "==> Building initramfs (${PROFILE})"

rm -rf "${STAGE}"
mkdir -p "${STAGE}"/{bin,sbin,etc,the-machine,run/the-machine,proc,sys,dev,tmp,var/log}

# Busybox provides /bin/sh and core utilities in the initramfs.
BUSYBOX="$(command -v busybox || true)"
if [[ -z "${BUSYBOX}" ]]; then
  echo "ERROR: busybox not found. Install busybox-static." >&2
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

SERVICES=(
  system-daemon
  mcp-bus
  policy-broker
  state-store
  event-bus
  lambda-server
  agent-core
  ui-runtime
  compositor
  fallback-shell
)

for svc in "${SERVICES[@]}"; do
  if [[ -x "${BIN_DIR}/${svc}" ]]; then
    install -m 0755 "${BIN_DIR}/${svc}" "${STAGE}/the-machine/${svc}"
  else
    echo "WARN: ${svc} binary not found at ${BIN_DIR}/${svc}" >&2
  fi
done

# Init script: start services in boot order (L0 → L3 → L1 → L4 → L5).
cat > "${STAGE}/init" <<'INIT'
#!/bin/sh
export PATH=/bin:/sbin:/the-machine
export RUST_LOG=info
export THE_MACHINE_SOCKET_DIR=/run/the-machine

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
sleep 1

# L4
start_svc agent-core
sleep 1

# L5
start_svc compositor
start_svc ui-runtime
start_svc fallback-shell

echo "[init] boot complete — spawning console"
exec /bin/sh
INIT
chmod +x "${STAGE}/init"

# Pack cpio archive.
mkdir -p "${BUILD_DIR}"
(
  cd "${STAGE}"
  find . | cpio -o -H newc 2>/dev/null | gzip -9 > "${OUTPUT}"
)

echo "==> Initramfs written to ${OUTPUT} ($(du -h "${OUTPUT}" | cut -f1))"
