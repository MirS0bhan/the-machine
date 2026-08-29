#!/usr/bin/env bash
# Fetch a small GGUF model for initramfs / ISO bundling (G11).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODEL_DIR="${ROOT}/build/models"
MODEL_PATH="${MODEL_DIR}/machine-tiny.gguf"
URL="${THE_MACHINE_MODEL_URL:-https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf}"

mkdir -p "${MODEL_DIR}"

if [[ -f "${MODEL_PATH}" ]]; then
  echo "==> Model already present: ${MODEL_PATH} ($(du -h "${MODEL_PATH}" | cut -f1))"
  exit 0
fi

if [[ "${THE_MACHINE_SKIP_MODEL_FETCH:-0}" == "1" ]]; then
  echo "==> Creating minimal GGUF placeholder (skip fetch)"
  # Minimal file so LOCAL_MODEL_PATH exists; native inference uses stub until real weights added.
  echo "GGUF_PLACEHOLDER" > "${MODEL_PATH}"
  exit 0
fi

echo "==> Fetching GGUF model to ${MODEL_PATH}"
echo "    URL: ${URL}"
if command -v curl >/dev/null 2>&1; then
  curl -L --retry 3 -o "${MODEL_PATH}" "${URL}"
elif command -v wget >/dev/null 2>&1; then
  wget -O "${MODEL_PATH}" "${URL}"
else
  echo "WARN: no curl/wget — writing placeholder" >&2
  echo "GGUF_PLACEHOLDER" > "${MODEL_PATH}"
fi

echo "==> Model ready: $(du -h "${MODEL_PATH}" | cut -f1)"
