#!/usr/bin/env bash
# Validate an installed rootfs mount for target-HW boot (G13 operator / CI).
# Usage: bash build/validate-installed-rootfs.sh /mnt/the-machine-root
set -euo pipefail

ROOTFS="${1:-}"
if [[ -z "${ROOTFS}" || ! -d "${ROOTFS}" ]]; then
  echo "Usage: $0 <installed-rootfs-mount>" >&2
  echo "Example: sudo mount /dev/sda1 /mnt && $0 /mnt" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export THE_MACHINE_VALIDATE_INSTALLED=1
exec bash "${SCRIPT_DIR}/rootfs-validate.sh" "${ROOTFS}"
