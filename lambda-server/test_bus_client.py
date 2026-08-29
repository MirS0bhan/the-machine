"""Tests for internal MCP bus registration helpers."""

import pytest

from bus_client import register_mcp_intent, register_event_handler


class TestBusClient:
  def test_register_mcp_intent_no_bus(self):
    """Should not raise when bus socket is unavailable (dev/test)."""
    assert register_mcp_intent("calc.eval", "calc.*") in (True, False)

  def test_register_event_handler_no_bus(self):
    assert register_event_handler("notifier", "task-complete.download") in (True, False)

  def test_event_key_parsing_category_pattern(self):
    # Smoke: ensures module imports and call path works
    result = register_event_handler("fn", "scheduler.heartbeat.tick")
    assert isinstance(result, bool)
