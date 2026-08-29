"""Data models for the Event/Scheduler Bus."""

from __future__ import annotations

from datetime import datetime, timezone
from typing import Any, Dict, Optional
from uuid import uuid4

from pydantic import BaseModel, Field


class EventPublishRequest(BaseModel):
    """Request to publish an event on the bus."""

    category: str
    pattern: str = "*"
    source: str = "unknown"
    payload: Dict[str, Any] = Field(default_factory=dict)
    requires_decision: bool = False


class EventRecord(BaseModel):
    """A published event record."""

    id: str = Field(default_factory=lambda: str(uuid4()))
    category: str
    pattern: str = "*"
    source: str = "unknown"
    payload: Dict[str, Any] = Field(default_factory=dict)
    timestamp: str = Field(
        default_factory=lambda: datetime.now(timezone.utc).isoformat()
    )
    requires_decision: bool = False
