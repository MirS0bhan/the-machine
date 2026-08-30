#!/usr/bin/env bash
# Copy glibc (and other) shared-library dependencies of a host binary into an initramfs stage.
# Without this, dynamically linked Rust binaries fail at runtime with ": not found".
set -euo pipefail

BIN="${1:?binary path required}"
STAGE="${2:?initramfs stage root required}"

if [[ ! -f "${BIN}" ]]; then
  echo "ERROR: binary not found: ${BIN}" >&2
  exit 1
fi

if ! ldd "${BIN}" >/dev/null 2>&1; then
  # Static binary or non-ELF — nothing to bundle.
  exit 0
fi

copy_lib() {
  local lib="$1"
  [[ -n "${lib}" && -f "${lib}" ]] || return 0
  local dest="${STAGE}${lib}"
  mkdir -p "$(dirname "${dest}")"
  cp -L "${lib}" "${dest}"
}

while IFS= read -r line; do
  case "${line}" in
    *'=> '/*)
      copy_lib "$(echo "${line}" | awk -F'=> ' '{print $2}' | awk '{print $1}')"
      ;;
    *ld-linux*)
      copy_lib "$(echo "${line}" | awk '{print $1}')"
      ;;
  esac
done < <(ldd "${BIN}" 2>/dev/null || true)
