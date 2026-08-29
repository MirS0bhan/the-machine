#!/usr/bin/env bash
# Collect per-component build outputs into a CI artifact directory.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-${ROOT}/build/artifacts}"
PROFILE="${2:-release}"
BIN_DIR="${ROOT}/target/${PROFILE}"

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

RUST_EXAMPLES=(
  fn-add
  fn-bad
)

PYTHON_PACKAGES=(
  lambda-server
  policy-broker
  state-store
  event-bus
  local-model
  ui-engine
)

echo "==> Packaging artifacts into ${OUT}"
rm -rf "${OUT}"
mkdir -p "${OUT}/rust" "${OUT}/rust/examples" "${OUT}/python/wheels" "${OUT}/iso"

# Rust service binaries
for svc in "${RUST_SERVICES[@]}"; do
  src="${BIN_DIR}/${svc}"
  if [[ -x "${src}" ]]; then
    install -m 0755 "${src}" "${OUT}/rust/${svc}"
    echo "  rust: ${svc}"
  else
    echo "  WARN: missing ${src}" >&2
  fi
done

# Lambda example binaries
for ex in "${RUST_EXAMPLES[@]}"; do
  src="${BIN_DIR}/${ex}"
  if [[ -x "${src}" ]]; then
    install -m 0755 "${src}" "${OUT}/rust/examples/${ex}"
    echo "  rust example: ${ex}"
  fi
done

# Python wheels (source trees packaged as wheels)
if command -v pip >/dev/null 2>&1; then
  for pkg in "${PYTHON_PACKAGES[@]}"; do
    echo "  python wheel: ${pkg}"
    pip wheel --no-deps -w "${OUT}/python/wheels" "${ROOT}/${pkg}" 2>/dev/null || \
      pip wheel --no-deps -w "${OUT}/python/wheels" "${ROOT}/${pkg}/" || true
  done
fi

# Boot artifacts
if [[ -f "${ROOT}/build/initramfs.cpio.gz" ]]; then
  cp "${ROOT}/build/initramfs.cpio.gz" "${OUT}/iso/initramfs.cpio.gz"
fi
if [[ -f "${ROOT}/build/the-machine.iso" ]]; then
  cp "${ROOT}/build/the-machine.iso" "${OUT}/iso/the-machine.iso"
fi

# Manifest
python3 "${ROOT}/build/ci-package-artifacts.py" "${OUT}"

echo "==> Artifact manifest: ${OUT}/manifest.json"
du -sh "${OUT}"/* 2>/dev/null || true
