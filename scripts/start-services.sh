#!/usr/bin/env bash
# Start all The Machine services in boot order (development harness).
#
# THE_MACHINE_RUNTIME controls which language implementation is used for
# overlapping components (see docs/guides/python-rust-overlap.md):
#   rust    — all Rust daemons (default, matches ISO boot)
#   hybrid  — Rust bus + daemons, Python policy-broker & lambda-server
#   python  — Python HTTP servers only (no Unix socket bus)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUNTIME="${THE_MACHINE_RUNTIME:-rust}"
SOCKET_DIR="${THE_MACHINE_SOCKET_DIR:-/tmp/the-machine/run}"
LOG_DIR="${THE_MACHINE_LOG_DIR:-/tmp/the-machine/log}"
PID_DIR="${THE_MACHINE_PID_DIR:-/tmp/the-machine/pid}"

mkdir -p "${SOCKET_DIR}" "${LOG_DIR}" "${PID_DIR}"
export THE_MACHINE_SOCKET_DIR="${SOCKET_DIR}"
export RUST_LOG="${RUST_LOG:-info}"

start_rust() {
  local name="$1"
  local bin="${ROOT}/target/debug/${name}"
  if [[ ! -x "${bin}" ]]; then
    echo "Building ${name}..."
    (cd "${ROOT}" && cargo build -p "${name}")
  fi
  echo "  [rust] ${name}"
  "${bin}" >"${LOG_DIR}/${name}.log" 2>&1 &
  echo $! > "${PID_DIR}/${name}.pid"
  sleep 0.5
}

start_python() {
  local name="$1"
  local cmd="$2"
  echo "  [python] ${name}"
  (cd "${ROOT}" && eval "${cmd}") >"${LOG_DIR}/${name}.log" 2>&1 &
  echo $! > "${PID_DIR}/${name}.pid"
  sleep 0.5
}

echo "==> The Machine service harness (runtime=${RUNTIME})"
echo "    sockets: ${SOCKET_DIR}"
echo "    logs:    ${LOG_DIR}"

case "${RUNTIME}" in
  python)
  echo "==> Python-only mode (HTTP servers, no socket bus)"
  start_python policy-broker \
    "cd policy-broker && uvicorn policy_broker.mcp_server:app --host 127.0.0.1 --port 8001"
  start_python state-store \
    "cd state-store && STATE_STORE_BACKEND=memory uvicorn state_store.mcp_server:app --port 8002"
  start_python lambda-server \
    "cd lambda-server && python3 -c 'from mcp_interface import MCPControlInterface; import time; MCPControlInterface(); time.sleep(999999)'"
  echo "==> Python services on ports 8001 (policy), 8002 (state)"
  echo "    Stop with: scripts/stop-services.sh"
  exit 0
  ;;
  hybrid|rust) ;;
  *)
  echo "ERROR: unknown THE_MACHINE_RUNTIME=${RUNTIME} (use rust|hybrid|python)" >&2
  exit 1
  ;;
esac

# Build all Rust binaries.
(cd "${ROOT}" && cargo build --workspace)

# L0
start_rust system-daemon

# L3
start_rust mcp-bus

# L2
if [[ "${RUNTIME}" == "hybrid" ]] && python3 -c "import policy_broker" 2>/dev/null; then
  start_python policy-broker \
    "cd policy-broker && uvicorn policy_broker.mcp_server:app --host 127.0.0.1 --port 8001"
else
  start_rust policy-broker
fi

# L1
start_rust state-store
start_rust event-bus

if [[ "${RUNTIME}" == "hybrid" ]]; then
  start_python lambda-server \
    "cd lambda-server && python3 -c 'from mcp_interface import MCPControlInterface; import time; MCPControlInterface(); time.sleep(999999)'"
else
  start_rust lambda-server
fi
start_rust local-model-daemon
start_rust marketplace

# L4
start_rust agent-core

# L5
start_rust compositor
start_rust ui-runtime
start_rust fallback-shell

echo "==> All services started. PIDs in ${PID_DIR}"
if [[ "${RUNTIME}" == "hybrid" ]]; then
  echo "    NOTE: policy-broker + lambda-server are Python (HTTP); rest are Rust (sockets)."
  echo "    They do NOT share state. See docs/guides/python-rust-overlap.md"
fi
echo "    Stop with: scripts/stop-services.sh"
