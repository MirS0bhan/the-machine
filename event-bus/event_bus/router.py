"""In-process event router used by integration tests and the MCP server.

This is a **test harness**, not a second production implementation.
The canonical event bus daemon is the Rust `event-bus` crate (scheduler, cron,
agent-wake coalescing). See `docs/guides/python-rust-overlap.md`.
"""

from __future__ import annotations

import asyncio
import logging
from typing import Any, Awaitable, Callable, Dict, List, Optional, Union
from uuid import uuid4

from event_bus.models import EventPublishRequest, EventRecord

logger = logging.getLogger(__name__)

EventHandler = Callable[[Dict[str, Any]], Union[None, Awaitable[None]]]


class EventRouter:
    """
    Lightweight event router with category-based subscriber dispatch.

    Production deployments use the Rust ``event-bus`` daemon; this Python
    implementation mirrors the routing contract for tests and local wiring.
    """

    def __init__(self) -> None:
        self.subscribers: Dict[str, List[EventHandler]] = {}
        self._handlers: Dict[str, EventHandler] = {}
        self._published: List[EventRecord] = []
        self._subscriptions: Dict[str, Dict[str, str]] = {}

    def register_handler(
        self,
        category: str,
        pattern: str,
        handler: EventHandler,
    ) -> None:
        key = f"{category}:{pattern}"
        self._handlers[key] = handler

    def publish(self, category: str, payload: Dict[str, Any]) -> EventRecord:
        """Publish an event to all subscribers of ``category``."""
        record = EventRecord(category=category, payload=payload)
        self._published.append(record)
        asyncio.create_task(self._dispatch(category, payload))
        return record

    def publish_request(self, request: EventPublishRequest) -> EventRecord:
        return self.publish(request.category, request.payload)

    def subscribe(
        self,
        category: str,
        handler: EventHandler,
        *,
        pattern: str = "*",
        subscriber: str = "anonymous",
    ) -> str:
        """Register a push subscription; returns ``subscription_id``."""
        sub_id = str(uuid4())
        self._subscriptions[sub_id] = {
            "category": category,
            "pattern": pattern,
            "subscriber": subscriber,
            "handler": handler,
        }
        self.subscribers.setdefault(category, []).append(handler)
        return sub_id

    def unsubscribe(self, subscription_id: str) -> bool:
        meta = self._subscriptions.pop(subscription_id, None)
        if meta is None:
            return False
        category = meta["category"]
        handler = meta["handler"]
        handlers = self.subscribers.get(category, [])
        if handler in handlers:
            handlers.remove(handler)
        if not handlers:
            self.subscribers.pop(category, None)
        return True

    async def _dispatch(self, category: str, payload: Dict[str, Any]) -> None:
        handlers = list(self.subscribers.get(category, []))
        for handler in handlers:
            try:
                result = handler(payload)
                if asyncio.iscoroutine(result):
                    await result
            except Exception:
                logger.exception("Event handler failed for category %s", category)

    def list_published(self) -> List[EventRecord]:
        return list(self._published)

    def clear(self) -> None:
        self.subscribers.clear()
        self._handlers.clear()
        self._published.clear()
        self._subscriptions.clear()
