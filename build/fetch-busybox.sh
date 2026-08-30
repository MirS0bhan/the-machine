#!/usr/bin/env bash
# Resolve a static busybox binary for initramfs assembly (mirrors fetch-model.sh).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CACHE_DIR="${ROOT}/build/tools"
CACHE="${CACHE_DIR}/busybox"

deb_arch() {
  case "$(uname -m)" in
    x86_64) echo amd64 ;;
    aarch64 | arm64) echo arm64 ;;
    armv7l | armv6l) echo armhf ;;
    i686 | i386) echo i386 ;;
    riscv64) echo riscv64 ;;
    ppc64le) echo ppc64el ;;
    s390x) echo s390x ;;
    *)
      echo "unsupported architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

default_deb_url() {
  local arch
  arch="$(deb_arch)"
  echo "http://ftp.debian.org/debian/pool/main/b/busybox/busybox-static_1.37.0-6+b9_${arch}.deb"
}

resolve_busybox() {
  if command -v busybox >/dev/null 2>&1; then
    command -v busybox
    return 0
  fi
  if [[ -x "${CACHE}" ]]; then
    echo "${CACHE}"
    return 0
  fi
  return 1
}

if path="$(resolve_busybox 2>/dev/null)"; then
  echo "==> Busybox ready: ${path} ($(${path} | head -1))"
  echo "${path}"
  exit 0
fi

if [[ "${THE_MACHINE_SKIP_BUSYBOX_FETCH:-0}" == "1" ]]; then
  echo "ERROR: busybox not found and THE_MACHINE_SKIP_BUSYBOX_FETCH=1" >&2
  exit 1
fi

URL="${THE_MACHINE_BUSYBOX_URL:-$(default_deb_url)}"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT

echo "==> Fetching static busybox to ${CACHE}"
echo "    URL: ${URL}"
mkdir -p "${CACHE_DIR}"

if command -v curl >/dev/null 2>&1; then
  curl -fsSL --retry 3 -o "${TMP}/busybox.deb" "${URL}"
elif command -v wget >/dev/null 2>&1; then
  wget -q -O "${TMP}/busybox.deb" "${URL}"
else
  echo "ERROR: need curl or wget to fetch busybox-static" >&2
  exit 1
fi

EXTRACT="${TMP}/extract"
mkdir -p "${EXTRACT}"
if command -v dpkg-deb >/dev/null 2>&1; then
  dpkg-deb -x "${TMP}/busybox.deb" "${EXTRACT}"
else
  (cd "${TMP}" && ar x busybox.deb)
  tar xf "${TMP}"/data.tar.* -C "${EXTRACT}"
fi

cp "${EXTRACT}/usr/bin/busybox" "${CACHE}"
chmod +x "${CACHE}"
echo "==> Busybox cached: ${CACHE} ($(${CACHE} | head -1))"
echo "${CACHE}"
