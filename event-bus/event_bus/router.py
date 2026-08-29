"""In-process event router used by integration tests and the MCP server."""

from __future__ import annotations

import asyncio
import logging
from typing import Any, Awaitable, Callable, Dict, List, Optional, Union

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
