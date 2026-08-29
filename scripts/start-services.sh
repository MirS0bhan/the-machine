#!/usr/bin/env bash
# Start all The Machine services in boot order (development harness).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
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
  echo "Starting ${name}"
  "${bin}" >"${LOG_DIR}/${name}.log" 2>&1 &
  echo $! > "${PID_DIR}/${name}.pid"
  sleep 0.5
}

start_python() {
  local pkg="$1"
  local cmd="$2"
  echo "Starting ${pkg} (Python)"
  (cd "${ROOT}/${pkg}" && eval "${cmd}") >"${LOG_DIR}/${pkg}.log" 2>&1 &
  echo $! > "${PID_DIR}/${pkg}.pid"
  sleep 0.5
}

echo "==> The Machine service harness"
echo "    sockets: ${SOCKET_DIR}"
echo "    logs:    ${LOG_DIR}"

# Build all Rust binaries first.
(cd "${ROOT}" && cargo build --workspace)

# L0
start_rust system-daemon

# L3
start_rust mcp-bus

# L2 — prefer Python policy broker (full rule engine) when available
if python3 -c "import policy_broker" 2>/dev/null; then
  start_python policy-broker "uvicorn policy_broker.mcp_server:app --host 127.0.0.1 --port 8001"
else
  start_rust policy-broker
fi

# L1
start_rust state-store
start_rust event-bus
if python3 -c "import sys; sys.path.insert(0, 'lambda-server'); import mcp_interface" 2>/dev/null; then
  start_python lambda-server "python3 -c 'from mcp_interface import MCPControlInterface; import time; MCPControlInterface(); time.sleep(999999)'"
else
  start_rust lambda-server
fi

# L4
start_rust agent-core

# L5
start_rust compositor
start_rust ui-runtime
start_rust fallback-shell

echo "==> All services started. PIDs in ${PID_DIR}"
echo "    Stop with: scripts/stop-services.sh"
