#!/usr/bin/env bash
# Fail if the ISO was built with a cloud-tuned kernel (blank VGA in QEMU).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PATH_FILE="${1:-${ROOT}/build/iso-kernel.path}"

if [[ ! -f "${PATH_FILE}" ]]; then
  echo "ERROR: kernel path file missing: ${PATH_FILE} (run make iso first)" >&2
  exit 1
fi

KERNEL="$(tr -d '\n' <"${PATH_FILE}")"
echo "==> ISO kernel: ${KERNEL}"

case "${KERNEL}" in
  *-azure*|*-aws*|*-gcp*|*-oracle*)
    echo "ERROR: ISO uses cloud-tuned kernel — QEMU -vga std will show a blank screen." >&2
    echo "Install linux-image-generic and rebuild without KERNEL= override." >&2
    exit 1
    ;;
esac

echo "==> ISO kernel OK (not cloud-tuned)"
