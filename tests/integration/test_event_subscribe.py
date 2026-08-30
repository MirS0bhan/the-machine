"""
Cross-component integration tests: Event Bus (`event.subscribe` → `event.publish`).

Workflow:
    1. Agent Core registers interest via `event.subscribe`.
    2. Policy Broker gates the call using `CAP_EVENT_ADMIN`.
    3. A publisher emits `event.publish` on the subscribed category.
    4. The subscriber handler receives the payload.

Assertions:
    - Policy Broker allows `event.subscribe` for permitted categories.
    - Subscribed handlers receive published payloads.
    - Category isolation prevents cross-delivery.
    - Default deny applies without a matching policy rule.
"""

import asyncio

import pytest
from policy_broker.models import CheckRequest


def _gate_event_subscribe(broker, category: str) -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_EVENT_ADMIN",
            path=category,
            method="event.subscribe",
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


class TestEventSubscribeIntegration:
    """End-to-end: subscribe → gate-check → publish → handler receives."""

    @pytest.mark.asyncio
    async def test_subscribe_then_publish_delivers_payload(
        self,
        event_bus,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "task-*",
                    "method": "event.subscribe",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_EVENT_ADMIN"],
                },
            ]
        )

        gate = _gate_event_subscribe(policy_broker, "task-complete")
        assert gate["allowed"], f"Broker DENY on subscribe: {gate['response'].message}"

        received = []

        async def handler(payload):
            received.append(payload)

        sub_id = event_bus.subscribe(
            "task-complete",
            handler,
            subscriber="agent-core",
        )
        assert sub_id

        event_bus.publish("task-complete", {"task_id": "t-sub-001", "status": "done"})
        await asyncio.sleep(0.05)

        assert len(received) == 1
        assert received[0]["task_id"] == "t-sub-001"
        assert received[0]["status"] == "done"

    @pytest.mark.asyncio
    async def test_subscribe_isolates_categories(
        self,
        event_bus,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "*",
                    "method": "event.subscribe",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_EVENT_ADMIN"],
                },
            ]
        )

        received_a = []
        received_b = []

        async def handler_a(payload):
            received_a.append(payload)

        async def handler_b(payload):
            received_b.append(payload)

        event_bus.subscribe("cat-a", handler_a, subscriber="svc-a")
        event_bus.subscribe("cat-b", handler_b, subscriber="svc-b")

        event_bus.publish("cat-a", {"msg": "only-a"})
        await asyncio.sleep(0.05)

        assert len(received_a) == 1
        assert len(received_b) == 0

    def test_deny_subscribe_without_policy(self, policy_broker):
        gate = _gate_event_subscribe(policy_broker, "task-complete")
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

    def test_deny_subscribe_for_restricted_category(
        self,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "task-*",
                    "method": "event.subscribe",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_EVENT_ADMIN"],
                },
            ]
        )

        gate = _gate_event_subscribe(policy_broker, "system-shutdown")
        assert not gate["allowed"]

    @pytest.mark.asyncio
    async def test_unsubscribe_stops_delivery(
        self,
        event_bus,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "*",
                    "method": "event.subscribe",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_EVENT_ADMIN"],
                },
            ]
        )

        received = []

        async def handler(payload):
            received.append(payload)

        sub_id = event_bus.subscribe("monitor", handler, subscriber="agent-core")
        event_bus.publish("monitor", {"seq": 1})
        await asyncio.sleep(0.05)
        assert len(received) == 1

        assert event_bus.unsubscribe(sub_id)
        event_bus.publish("monitor", {"seq": 2})
        await asyncio.sleep(0.05)
        assert len(received) == 1
        assert received[0]["seq"] == 1
