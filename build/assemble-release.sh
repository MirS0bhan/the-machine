#!/usr/bin/env bash
# Assemble a combined release directory from GitHub Actions artifacts
# downloaded into ARTIFACTS_IN (one subdirectory per artifact name).
#
# Usage: assemble-release.sh <artifacts-in-dir> <out-dir>
set -euo pipefail

ARTIFACTS_IN="${1:?artifacts-in directory required}"
OUT="${2:?output directory required}"

# Service binaries uploaded as rust-<name>/<name>. Do NOT glob rust-*:
# jobs such as coverage upload rust-coverage (an lcov file), which is not
# a component binary. See CI failure on main after PR #6.
RUST_SERVICES=(
  system-daemon
  mcp-bus
  policy-broker
  state-store
  event-bus
  lambda-server
  local-model-daemon
  marketplace
  agent-core
  ui-runtime
  compositor
  fallback-shell
)

mkdir -p "${OUT}/rust" "${OUT}/rust/examples" "${OUT}/python/wheels" "${OUT}/iso"

missing=0
for comp in "${RUST_SERVICES[@]}"; do
  src="${ARTIFACTS_IN}/rust-${comp}/${comp}"
  if [[ -f "${src}" ]]; then
    install -m 0755 "${src}" "${OUT}/rust/${comp}"
  else
    echo "ERROR: missing Rust binary artifact rust-${comp}/${comp}" >&2
    missing=1
  fi
done
if [[ "${missing}" -ne 0 ]]; then
  exit 1
fi

if [[ -d "${ARTIFACTS_IN}/rust-lambda-examples" ]]; then
  install -m 0755 "${ARTIFACTS_IN}/rust-lambda-examples/fn-add" "${OUT}/rust/examples/fn-add"
  install -m 0755 "${ARTIFACTS_IN}/rust-lambda-examples/fn-bad" "${OUT}/rust/examples/fn-bad"
fi

shopt -s nullglob
for dir in "${ARTIFACTS_IN}"/python-*/; do
  cp "${dir}"*.whl "${OUT}/python/wheels/"
done
shopt -u nullglob

install -m 0644 "${ARTIFACTS_IN}/boot-initramfs/initramfs.cpio.gz" "${OUT}/iso/initramfs.cpio.gz"
install -m 0644 "${ARTIFACTS_IN}/boot-iso/the-machine.iso" "${OUT}/iso/the-machine.iso"

# Optional reports (never treated as binaries)
if [[ -f "${ARTIFACTS_IN}/coverage-lcov/coverage.lcov" ]]; then
  mkdir -p "${OUT}/coverage"
  install -m 0644 "${ARTIFACTS_IN}/coverage-lcov/coverage.lcov" "${OUT}/coverage/coverage.lcov"
elif [[ -f "${ARTIFACTS_IN}/rust-coverage/coverage.lcov" ]]; then
  mkdir -p "${OUT}/coverage"
  install -m 0644 "${ARTIFACTS_IN}/rust-coverage/coverage.lcov" "${OUT}/coverage/coverage.lcov"
fi
