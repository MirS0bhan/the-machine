"""
Cross-component integration tests: Event Bus → Agent Core (wake signals).

Workflow:
    1. Lambda Server publishes a `task-complete` event via `event.publish`.
    2. Policy Broker gates the call using `CAP_EVENT_PUBLISH`.
    3. Event Bus routes the event to the Agent Core's subscriber.
    4. Agent Core processes the event (stub handler).

Assertions:
    - Policy Broker allows `event.publish` for `task-complete` events.
    - Event Bus routes the event to the Agent Core.
    - Agent Core subscriber receives the event payload.
"""

import asyncio
import pytest
from policy_broker.models import CheckRequest


# ── helpers ────────────────────────────────────────────────────────────

def _gate_event_publish(broker, category: str) -> dict:
    resp = broker.check(CheckRequest(
        capability="CAP_EVENT_PUBLISH",
        path=category,
        method="event.publish",
        principal="lambda-server",
        provenance="task-execution",
    ))
    return {"allowed": resp.decision == "ALLOW", "response": resp}


# ── tests ──────────────────────────────────────────────────────────────


class TestEventAgentWakeSignals:
    """End-to-end: publish → gate-check → subscriber receives → assert.

    EventBus.publish() uses asyncio.create_task(), so all publish-side
    tests must run inside an async context.
    """

    @pytest.mark.asyncio
    async def test_task_complete_wakes_agent_core(
        self,
        event_bus,
        policy_broker,
        state_store,
        register_policy,
    ):
        register_policy([
            {
                "path": "*",
                "method": "event.publish",
                "decision": "ALLOW",
                "capabilities": ["CAP_EVENT_PUBLISH"],
            },
        ])

        received_events = []

        async def agent_core_handler(payload):
            received_events.append(payload)

        event_bus.subscribers["task-complete"] = [agent_core_handler]

        gate = _gate_event_publish(policy_broker, "task-complete")
        assert gate["allowed"], f"Broker DENY: {gate['response'].message}"

        payload = {"task_id": "t-001", "result": "done", "duration_ms": 120}
        event_bus.publish("task-complete", payload)

        await asyncio.sleep(0.05)

        assert len(received_events) == 1
        assert received_events[0]["task_id"] == "t-001"
        assert received_events[0]["result"] == "done"

    @pytest.mark.asyncio
    async def test_multiple_subscribers_receive_event(
        self,
        event_bus,
    ):
        received_a = []
        received_b = []

        async def handler_a(payload):
            received_a.append(payload)

        async def handler_b(payload):
            received_b.append(payload)

        event_bus.subscribers["task-complete"] = [handler_a, handler_b]

        event_bus.publish("task-complete", {"task_id": "t-002"})

        await asyncio.sleep(0.05)

        assert len(received_a) == 1
        assert len(received_b) == 1
        assert received_a[0]["task_id"] == "t-002"
        assert received_b[0]["task_id"] == "t-002"

    @pytest.mark.asyncio
    async def test_event_category_isolation(
        self,
        event_bus,
    ):
        """An event on category A does not reach subscribers of category B."""
        received_a = []
        received_b = []

        async def handler_a(payload):
            received_a.append(payload)

        async def handler_b(payload):
            received_b.append(payload)

        event_bus.subscribers["cat-a"] = [handler_a]
        event_bus.subscribers["cat-b"] = [handler_b]

        event_bus.publish("cat-a", {"msg": "hello-a"})

        await asyncio.sleep(0.05)

        assert len(received_a) == 1
        assert len(received_b) == 0

    def test_deny_publish_without_policy(
        self,
        policy_broker,
    ):
        gate = _gate_event_publish(policy_broker, "task-complete")
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

    def test_deny_publish_for_restricted_category(
        self,
        policy_broker,
        register_policy,
    ):
        register_policy([
            {
                "path": "task-*",
                "method": "event.publish",
                "decision": "ALLOW",
                "capabilities": ["CAP_EVENT_PUBLISH"],
            },
        ])

        gate = _gate_event_publish(policy_broker, "system-shutdown")
        assert not gate["allowed"]

    @pytest.mark.asyncio
    async def test_event_bus_publish_no_exception(
        self,
        event_bus,
    ):
        """The publish call itself should not raise."""
        received = []

        async def handler(payload):
            received.append(payload)

        event_bus.subscribers["monitor"] = [handler]

        event_bus.publish("monitor", {"cpu": 42.5})
        await asyncio.sleep(0.05)

        assert len(received) == 1
        assert received[0]["cpu"] == 42.5

    @pytest.mark.asyncio
    async def test_event_payload_preserved_end_to_end(
        self,
        event_bus,
    ):
        """Complex nested payloads survive the publish/subscribe round trip."""
        received = []

        async def handler(payload):
            received.append(payload)

        event_bus.subscribers["complex"] = [handler]

        payload = {
            "task_id": "t-complex",
            "metadata": {"version": 3, "tags": ["alpha", "beta"]},
            "result": {"nested": {"deep": True}},
        }
        event_bus.publish("complex", payload)

        await asyncio.sleep(0.05)

        assert received[0] == payload
