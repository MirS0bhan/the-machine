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
for app in sh ls cat echo mkdir mount umount sleep grep sed awk modprobe insmod depmod; do
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
    bash "${ROOT}/build/bundle-shared-libs.sh" "${BIN_DIR}/${svc}" "${STAGE}"
  else
    echo "WARN: ${svc} binary not found at ${BIN_DIR}/${svc}" >&2
  fi
done

ISO_KERNEL="${KERNEL:-$(bash "${ROOT}/build/select-kernel.sh" 2>/dev/null || true)}"
KVER="$(uname -r)"
if [[ -n "${ISO_KERNEL}" ]]; then
  KVER="$(basename "${ISO_KERNEL}" | sed 's/^vmlinuz-//')"
fi
bash "${ROOT}/build/bundle-kernel-modules.sh" "${STAGE}" "${KVER}" || true

if compgen -G "${STAGE}/the-machine/*" >/dev/null && \
   ldd "${STAGE}/the-machine/"* >/dev/null 2>&1; then
  echo "==> Bundled shared libraries for dynamic Rust binaries"
fi

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
if [[ -d "${ROOT}/assets/fonts" ]]; then
  mkdir -p "${STAGE}/etc/the-machine/fonts" "${STAGE}/the-machine/fonts"
  cp "${ROOT}/assets/fonts/"*.ttf "${STAGE}/etc/the-machine/fonts/" 2>/dev/null || true
  cp "${ROOT}/assets/fonts/"*.ttf "${STAGE}/the-machine/fonts/" 2>/dev/null || true
fi
# Locale catalogs (i18n) — ui-runtime reads /etc/the-machine/locales at runtime.
if [[ -d "${ROOT}/assets/locales" ]]; then
  mkdir -p "${STAGE}/etc/the-machine/locales"
  cp "${ROOT}/assets/locales/"*.json "${STAGE}/etc/the-machine/locales/" 2>/dev/null || true
fi

# Boot logging + PID 1 init script.
install -m 0644 "${ROOT}/build/boot-log-lib.sh" "${STAGE}/boot-log-lib.sh"
install -m 0755 "${ROOT}/build/boot-init.sh" "${STAGE}/init"
install -m 0755 "${ROOT}/build/collect-boot-logs.sh" "${STAGE}/collect-boot-logs.sh"
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
