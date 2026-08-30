#!/usr/bin/env bash
# Pick a host kernel for the bootable ISO.
# Cloud-tuned kernels (azure, aws, gcp) often lack QEMU std-VGA / bochs-drm drivers.
set -euo pipefail

pick_kernel() {
  local k flavor
  local -a candidates=()
  local -a generic=()
  local -a other=()
  local -a cloud=()

  shopt -s nullglob
  for k in /boot/vmlinuz-*; do
    [[ -f "$k" ]] || continue
    flavor="${k#/boot/vmlinuz-}"
    case "$flavor" in
      *-azure|*-aws|*-gcp|*-oracle|*-oem)
        cloud+=("$k")
        ;;
      *-generic|*-generic-hwe)
        generic+=("$k")
        ;;
      *)
        other+=("$k")
        ;;
    esac
  done
  shopt -u nullglob

  if ((${#generic[@]})); then
    printf '%s\n' "${generic[@]}" | sort -V | tail -1
    return 0
  fi
  if ((${#other[@]})); then
    printf '%s\n' "${other[@]}" | sort -V | tail -1
    return 0
  fi
  if ((${#cloud[@]})); then
    printf '%s\n' "${cloud[@]}" | sort -V | tail -1
    return 0
  fi
  return 1
}

if [[ "${1:-}" == "--warn-if-cloud" ]]; then
  k="${2:-}"
  if [[ "$k" == *-azure* || "$k" == *-aws* || "$k" == *-gcp* ]]; then
    echo "WARN: kernel $(basename "$k") is cloud-tuned; QEMU -vga std may show a blank screen." >&2
    echo "WARN: install linux-image-generic and rebuild: KERNEL=/boot/vmlinuz-*-generic make iso" >&2
  fi
  exit 0
fi

pick_kernel
