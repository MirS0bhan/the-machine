#!/usr/bin/env bash
# Regression test: rust-coverage (or any non-binary rust-* artifact) must not
# be treated as a service binary named "coverage".
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT

IN="${TMP}/in"
OUT="${TMP}/out"

SERVICES=(
  system-daemon mcp-bus policy-broker state-store event-bus
  lambda-server local-model-daemon marketplace agent-core
  ui-runtime compositor fallback-shell
)

for comp in "${SERVICES[@]}"; do
  mkdir -p "${IN}/rust-${comp}"
  echo "bin-${comp}" > "${IN}/rust-${comp}/${comp}"
  chmod +x "${IN}/rust-${comp}/${comp}"
done

mkdir -p "${IN}/rust-lambda-examples" "${IN}/python-lambda-server" \
         "${IN}/boot-initramfs" "${IN}/boot-iso" "${IN}/rust-coverage"
echo add > "${IN}/rust-lambda-examples/fn-add"
echo bad > "${IN}/rust-lambda-examples/fn-bad"
chmod +x "${IN}/rust-lambda-examples/fn-add" "${IN}/rust-lambda-examples/fn-bad"
echo "wheel" > "${IN}/python-lambda-server/lambda_server-0.1-py3-none-any.whl"
echo initrd > "${IN}/boot-initramfs/initramfs.cpio.gz"
echo iso > "${IN}/boot-iso/the-machine.iso"
echo "TN:fake" > "${IN}/rust-coverage/coverage.lcov"

bash "${ROOT}/build/assemble-release.sh" "${IN}" "${OUT}"

# Must not invent a "coverage" binary from the rust-coverage artifact.
if [[ -e "${OUT}/rust/coverage" ]]; then
  echo "FAIL: rust-coverage was installed as a binary" >&2
  exit 1
fi

for comp in "${SERVICES[@]}"; do
  [[ -f "${OUT}/rust/${comp}" ]] || { echo "FAIL: missing ${comp}" >&2; exit 1; }
done
[[ -f "${OUT}/rust/examples/fn-add" ]] || { echo "FAIL: missing fn-add" >&2; exit 1; }
[[ -f "${OUT}/iso/the-machine.iso" ]] || { echo "FAIL: missing ISO" >&2; exit 1; }
[[ -f "${OUT}/coverage/coverage.lcov" ]] || { echo "FAIL: coverage report not copied" >&2; exit 1; }

# Missing required binary must fail (not silently skip).
rm -rf "${IN}/rust-mcp-bus"
if bash "${ROOT}/build/assemble-release.sh" "${IN}" "${OUT}-fail" 2>/dev/null; then
  echo "FAIL: assemble succeeded with missing mcp-bus" >&2
  exit 1
fi

echo "OK: assemble-release ignores rust-coverage and requirelists binaries"
