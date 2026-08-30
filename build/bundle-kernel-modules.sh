#!/usr/bin/env bash
# Copy display-related kernel modules into the initramfs (QEMU virtio-gpu / cirrusfb).
set -euo pipefail

STAGE="${1:?initramfs stage root required}"
KVER="${2:-$(uname -r)}"
SRC="/lib/modules/${KVER}"

if [[ ! -d "${SRC}" ]]; then
  echo "WARN: ${SRC} missing — skip kernel module bundle (VGA will use memory backend)" >&2
  exit 0
fi

DEST="${STAGE}/lib/modules/${KVER}"
mkdir -p "${DEST}/kernel"

find_module_ko() {
  local name="$1"
  local stem="${name//_/-}"
  find "${SRC}/kernel" \( -name "${stem}.ko" -o -name "${stem}.ko.zst" \
    -o -name "${name}.ko" -o -name "${name}.ko.zst" \) 2>/dev/null | head -1
}

copy_ko() {
  local ko="$1"
  [[ -n "${ko}" && -f "${ko}" ]] || return 0
  local rel="${ko#${SRC}/}"
  local out="${DEST}/${rel}"
  mkdir -p "$(dirname "${out}")"
  if [[ "${ko}" == *.ko.zst ]]; then
  if command -v zstd >/dev/null 2>&1; then
    zstd -d -q -f -o "${out%.zst}" "${ko}"
  else
    echo "WARN: zstd not found — cannot decompress ${ko}" >&2
  fi
  else
    cp -a "${ko}" "${out}"
  fi
}

# Prefer virtio-gpu (QEMU -device virtio-vga), then legacy VGA drivers if present.
DISPLAY_MODULES=(virtio_gpu virtio_dma_buf bochs cirrusfb drm drm_kms_helper ttm)

copied=0
for mod in "${DISPLAY_MODULES[@]}"; do
  ko="$(find_module_ko "${mod}")"
  if [[ -n "${ko}" ]]; then
    copy_ko "${ko}"
    copied=$((copied + 1))
  fi
done

if [[ "${copied}" -eq 0 ]]; then
  echo "WARN: no display modules found under ${SRC}/kernel" >&2
  exit 0
fi

for meta in modules.dep modules.alias modules.symbols modules.softdep modules.builtin modules.builtin.modinfo; do
  if [[ -f "${SRC}/${meta}" ]]; then
    cp -a "${SRC}/${meta}" "${DEST}/${meta}"
  fi
done

if command -v depmod >/dev/null 2>&1; then
  depmod -b "${STAGE}" "${KVER}" 2>/dev/null || true
fi

echo "==> Bundled ${copied} kernel module(s) for ${KVER} display"
