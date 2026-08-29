#!/usr/bin/env bash
# Bidirectional documentation ↔ code verification.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

python3 "${ROOT}/scripts/verify-docs-code.py"
