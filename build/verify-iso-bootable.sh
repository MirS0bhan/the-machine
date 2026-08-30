#!/usr/bin/env bash
# Fail unless PATH is a bootable ISO 9660 image (El Torito / GRUB hybrid).
set -euo pipefail

ISO="${1:?iso path required}"

if [[ ! -f "${ISO}" ]]; then
  echo "ERROR: ISO not found: ${ISO}" >&2
  exit 1
fi

desc="$(file -b "${ISO}")"
if [[ "${desc}" != *bootable* ]]; then
  echo "ERROR: ${ISO} is not bootable: ${desc}" >&2
  echo "Hint: install grub-pc-bin, xorriso, and mtools; ensure kernel files in the ISO tree are world-readable." >&2
  exit 1
fi

echo "==> Verified bootable ISO: ${desc}"
