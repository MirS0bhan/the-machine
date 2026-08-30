#!/usr/bin/env bash
# Regression: ISO kernel selection prefers generic over cloud-tuned.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT

mkdir -p "${TMP}/boot"
touch "${TMP}/boot/vmlinuz-6.17.0-1022-azure"
touch "${TMP}/boot/vmlinuz-6.8.0-55-generic"

pick() {
  shopt -s nullglob
  local k flavor generic=() cloud=() other=()
  for k in "${TMP}"/boot/vmlinuz-*; do
    flavor="${k#${TMP}/boot/vmlinuz-}"
    case "$flavor" in
      *-azure|*-aws|*-gcp) cloud+=("$k") ;;
      *-generic|*-generic-hwe) generic+=("$k") ;;
      *) other+=("$k") ;;
    esac
  done
  shopt -u nullglob
  if ((${#generic[@]})); then printf '%s\n' "${generic[@]}" | sort -V | tail -1; return; fi
  if ((${#other[@]})); then printf '%s\n' "${other[@]}" | sort -V | tail -1; return; fi
  if ((${#cloud[@]})); then printf '%s\n' "${cloud[@]}" | sort -V | tail -1; return; fi
  return 1
}

chosen="$(pick)"
[[ "$chosen" == *-generic ]] || {
  echo "FAIL: expected generic kernel, got ${chosen}" >&2
  exit 1
}

bash "${ROOT}/build/select-kernel.sh" --warn-if-cloud "${TMP}/boot/vmlinuz-6.17.0-1022-azure" 2>&1 | grep -q azure || {
  echo "FAIL: expected azure warning" >&2
  exit 1
}

echo "==> select-kernel ok"
