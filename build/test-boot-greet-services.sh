#!/usr/bin/env bash
# Live service integration: boot AUIL → agent greet → chat send.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASE="${THE_MACHINE_TEST_BASE:-/tmp/the-machine-boot-greet-test}"
SOCKET_DIR="${BASE}/run"
PID_DIR="${BASE}/pid"
LOG_DIR="${BASE}/log"

export THE_MACHINE_SOCKET_DIR="${SOCKET_DIR}"
export THE_MACHINE_PID_DIR="${PID_DIR}"
export THE_MACHINE_LOG_DIR="${LOG_DIR}"
export THE_MACHINE_COMPOSITOR_BACKEND=memory
export THE_MACHINE_COMPOSITOR_STATIC=1
export THE_MACHINE_FB_WIDTH=640
export THE_MACHINE_FB_HEIGHT=360
export THE_MACHINE_BOOT_AUIL="${ROOT}/build/boot.auil"
export THE_MACHINE_POLICY_FAIL_OPEN=1

cleanup() {
  bash "${ROOT}/scripts/stop-services.sh" >/dev/null 2>&1 || true
  pkill -f "${ROOT}/target/debug/" >/dev/null 2>&1 || true
}
trap cleanup EXIT

pkill -f "${ROOT}/target/debug/" >/dev/null 2>&1 || true
sleep 1
rm -rf "${BASE}"
mkdir -p "${SOCKET_DIR}" "${PID_DIR}" "${LOG_DIR}"

echo "==> boot greet services: start harness (${SOCKET_DIR})"
bash "${ROOT}/scripts/start-services.sh" >/dev/null
sleep 5

echo "==> boot greet services: wait for compositor + agent greet"
python3 - <<'PY'
import json
import os
import socket
import sys
import time
import uuid
from pathlib import Path

sock_path = Path(os.environ["THE_MACHINE_SOCKET_DIR"]) / "mcp-bus.sock"


def mcp(method: str, params: dict | None = None) -> dict:
    req = {
        "id": str(uuid.uuid4()),
        "kind": "Request",
        "method": method,
        "params": params or {},
    }
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
        s.settimeout(10)
        s.connect(str(sock_path))
        s.sendall((json.dumps(req) + "\n").encode())
        data = s.recv(65536)
    return json.loads(data.decode())


def wait_for(pred, label: str, timeout: float = 60.0) -> None:
    deadline = time.time() + timeout
    last = None
    while time.time() < deadline:
        try:
            if pred():
                return
        except (FileNotFoundError, ConnectionRefusedError, json.JSONDecodeError, TimeoutError) as exc:
            last = exc
        time.sleep(0.5)
    raise SystemExit(f"timeout waiting for {label}: {last}")


wait_for(
    lambda: mcp("compositor.status").get("result", {}).get("status") == "running",
    "compositor.status",
)

wait_for(
    lambda: mcp("ui.status").get("result", {}).get("status") == "running",
    "ui.status",
)

def greeted() -> bool:
    r = mcp("ui.get", {"id": "ui.greeting"})
    text = r.get("result", {}).get("props", {}).get("text", "")
    return "Hello" in text or "Machine" in text

wait_for(greeted, "agent boot.greet ui.patch")

mcp(
    "ui.patch",
    {
        "ops": [
            {
                "op": "update",
                "id": "ui.chat_input",
                "props": {"text": "hello e2e"},
            }
        ]
    },
)
mcp("ui.event", {"id": "ui.chat_send", "event": "press", "payload": {}})


def chat_updated() -> bool:
    r = mcp("ui.get", {"id": "ui.chat_log"})
    text = r.get("result", {}).get("props", {}).get("text", "")
    return "You: hello e2e" in text


wait_for(chat_updated, "chat.message ui.patch")

status = mcp("agent.status").get("result", {})
if int(status.get("wakes_processed", 0)) < 1:
    raise SystemExit(f"agent did not process wakes: {status}")

print("boot greet services integration passed")
PY

echo "==> boot greet services passed"
