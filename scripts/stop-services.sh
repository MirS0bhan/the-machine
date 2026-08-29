#!/usr/bin/env bash
# Stop all services started by start-services.sh
set -euo pipefail

PID_DIR="${THE_MACHINE_PID_DIR:-/tmp/the-machine/pid}"

if [[ ! -d "${PID_DIR}" ]]; then
  echo "No PID directory at ${PID_DIR}"
  exit 0
fi

for f in "${PID_DIR}"/*.pid; do
  [[ -f "$f" ]] || continue
  pid=$(cat "$f")
  name=$(basename "$f" .pid)
  if kill -0 "$pid" 2>/dev/null; then
    echo "Stopping ${name} (pid ${pid})"
    kill "$pid" 2>/dev/null || true
  fi
  rm -f "$f"
done

echo "Done."
