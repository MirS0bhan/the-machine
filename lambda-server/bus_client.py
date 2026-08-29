"""
Internal MCP Bus registration helpers for the Lambda Server.

Registration is a side effect of lambda.register (mcp-bus-spec.md §3) — not
exposed to agents. Routes hot-register `exposes_mcp` (mcp-intent namespace)
and `handles_event` (event-handler namespace + event-bus handler table).
"""

from __future__ import annotations

import json
import logging
import os
import socket
from typing import Any, Dict, Optional

logger = logging.getLogger(__name__)


def _socket_path() -> str:
    base = os.environ.get("THE_MACHINE_SOCKET_DIR", "/run/the-machine")
    return f"{base}/mcp-bus.sock"


def _bus_call(method: str, params: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    """Best-effort one-shot MCP call to the bus (no-op if bus unavailable)."""
    path = _socket_path()
    if not os.path.exists(os.path.dirname(path)):
        return None
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(2.0)
        sock.connect(path)
        req = json.dumps({"id": 1, "kind": "Request", "method": method, "params": params})
        sock.sendall(req.encode() + b"\n")
        data = sock.recv(65536)
        sock.close()
        if not data:
            return None
        resp = json.loads(data.decode())
        return resp.get("result")
    except OSError as e:
        logger.debug("bus call %s skipped: %s", method, e)
        return None


def register_mcp_intent(lambda_name: str, pattern: str) -> bool:
    """Register an mcp-intent route after lambda.register."""
    result = _bus_call(
        "_bus.register",
        {
            "namespace": "mcp-intent",
            "pattern": pattern,
            "handler": "lambda-server",
            "registered_by": "lambda-server",
            "manifest_ref": lambda_name,
        },
    )
    if result:
        logger.info("bus route: %s → lambda-server (%s)", pattern, lambda_name)
    return result is not None


def register_event_handler(lambda_name: str, event_key: str) -> bool:
    """
    Register handles_event: bus event-handler namespace + event-bus routing table.

    event_key format: ``category.pattern`` (e.g. ``task-complete.download``).
    """
    if "." in event_key:
        category, pattern = event_key.split(".", 1)
    else:
        category, pattern = event_key, "*"

    bus_ok = _bus_call(
        "_bus.register",
        {
            "namespace": "event-handler",
            "pattern": event_key,
            "handler": "lambda-server",
            "registered_by": "lambda-server",
            "manifest_ref": lambda_name,
        },
    )

    event_path = os.environ.get("THE_MACHINE_SOCKET_DIR", "/run/the-machine")
    event_sock = f"{event_path}/event-bus.sock"
    event_ok = False
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(2.0)
        sock.connect(event_sock)
        req = json.dumps(
            {
                "id": 2,
                "kind": "Request",
                "method": "event.register_handler",
                "params": {
                    "category": category,
                    "pattern": pattern,
                    "handler": "lambda-server",
                },
            }
        )
        sock.sendall(req.encode() + b"\n")
        data = sock.recv(65536)
        sock.close()
        event_ok = bool(data)
    except OSError as e:
        logger.debug("event.register_handler skipped: %s", e)

    if bus_ok or event_ok:
        logger.info("event handler: %s → lambda-server (%s)", event_key, lambda_name)
    return bus_ok is not None or event_ok
