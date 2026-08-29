"""MCP server entry point for the Python Event Bus (test/dev harness)."""

from __future__ import annotations

import json
import logging
from typing import Any, Dict

from event_bus.models import EventPublishRequest
from event_bus.router import EventRouter

logger = logging.getLogger(__name__)

_router = EventRouter()


def handle_mcp(method: str, params: Dict[str, Any]) -> Dict[str, Any]:
    if method in ("event.publish", "event.emit"):
        req = EventPublishRequest(**params)
        record = _router.publish_request(req)
        return {"success": True, "event_id": record.id}
    if method == "event.stats":
        return {
            "published": len(_router.list_published()),
            "subscribers": {k: len(v) for k, v in _router.subscribers.items()},
        }
    return {"success": False, "error": f"Unknown method: {method}"}


def main() -> None:
    logging.basicConfig(level=logging.INFO)
    logger.info("Event Bus MCP server (Python harness) — stdin/stdout JSON-RPC")
    for line in __import__("sys").stdin:
        line = line.strip()
        if not line:
            continue
        msg = json.loads(line)
        method = msg.get("method", "")
        params = msg.get("params") or {}
        result = handle_mcp(method, params)
        print(json.dumps({"id": msg.get("id"), "result": result}), flush=True)


if __name__ == "__main__":
    main()
