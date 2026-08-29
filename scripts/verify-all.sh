#!/usr/bin/env bash
# Full verification: tests, builds, docs, and component inventory vs documentation.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓${NC} $*"; }
fail() { echo -e "${RED}✗${NC} $*"; exit 1; }

echo "==> The Machine — full verification"
echo

# --- 1. Rust workspace build ---
echo "--- Rust build (release) ---"
cargo build --workspace --release
pass "Rust workspace builds"

RUST_BINS=(
  system-daemon mcp-bus policy-broker state-store event-bus
  lambda-server agent-core ui-runtime compositor fallback-shell
)
for bin in "${RUST_BINS[@]}"; do
  [[ -x "target/release/${bin}" ]] || fail "Missing Rust binary: ${bin}"
done
pass "All ${#RUST_BINS[@]} Rust service binaries present"

# --- 2. Python packages install ---
echo "--- Python packages ---"
pip install -q pytest pytest-asyncio pytest-mock build wheel markdown
pip install -q -e lambda-server -e policy-broker -e state-store -e event-bus -e ui-engine
pip install -q -e local-model --no-deps
pip install -q fastapi uvicorn pydantic python-multipart
pass "Python packages installed"

# --- 3. Tests ---
echo "--- Tests ---"
make test-all
pass "All tests passed"

# --- 4. Python wheels ---
echo "--- Python wheels ---"
mkdir -p build/verify-wheels
for pkg in lambda-server policy-broker state-store event-bus local-model ui-engine; do
  pip wheel --no-deps -w build/verify-wheels "${pkg}/" -q
done
WHEEL_COUNT=$(ls -1 build/verify-wheels/*.whl 2>/dev/null | wc -l)
[[ "${WHEEL_COUNT}" -ge 6 ]] || fail "Expected 6 Python wheels, got ${WHEEL_COUNT}"
pass "${WHEEL_COUNT} Python wheels built"

# --- 5. Docs build ---
echo "--- Documentation ---"
make docs
[[ -f docs/build/index.html ]] || fail "docs/build/index.html missing"
pass "Documentation built"

# --- 6. Component inventory vs docs ---
echo "--- Component inventory ---"
python3 - <<'PY'
import json, sys
from pathlib import Path

ROOT = Path(".")
errors = []

RUST = [
    "system-daemon", "mcp-bus", "policy-broker", "state-store", "event-bus",
    "lambda-server", "agent-core", "ui-runtime", "compositor", "fallback-shell",
]
PYTHON = ["lambda-server", "policy-broker", "state-store", "event-bus", "local-model", "ui-engine"]

for b in RUST:
    p = ROOT / "target/release" / b
    if not p.exists():
        errors.append(f"Rust binary missing: {b}")

for pkg in PYTHON:
  pyproject = ROOT / pkg / "pyproject.toml"
  if not pyproject.exists():
    errors.append(f"Python package missing pyproject.toml: {pkg}")

overlap_doc = ROOT / "docs/guides/python-rust-overlap.md"
if not overlap_doc.exists():
    errors.append("Missing docs/guides/python-rust-overlap.md")

book = (ROOT / "docs/book/index.md").read_text()
if "agent/" in book and "agent-core/" not in book.split("agent/")[0][-20:]:
    if "├── agent/" in book:
        errors.append("docs/book/index.md still references removed agent/ directory")

if errors:
    for e in errors:
        print(f"  ✗ {e}", file=sys.stderr)
    sys.exit(1)
print(f"  {len(RUST)} Rust + {len(PYTHON)} Python components verified")
PY
pass "Component inventory matches documentation"

# --- 7. Initramfs (release) ---
echo "--- Initramfs ---"
make initramfs-release
[[ -f build/initramfs.cpio.gz ]] || fail "initramfs.cpio.gz missing"
pass "Release initramfs built ($(du -h build/initramfs.cpio.gz | cut -f1))"

echo
echo "=========================================="
pass "FULL VERIFICATION PASSED"
echo "=========================================="
