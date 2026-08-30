"""
Cross-component integration tests: Event Bus (`event.schedule` policy gating).

Workflow:
    1. Lambda Server requests `event.schedule` with a cron/when expression.
    2. Policy Broker gates the call using `CAP_TIMER`.
    3. On allow, a scheduled timer would be registered in the Rust event-bus daemon.
    4. When the timer fires, the bus publishes to the target category (simulated here).

Assertions:
    - Policy Broker allows `event.schedule` when `CAP_TIMER` is granted.
    - Default deny applies without a matching policy rule.
    - Scheduled timer delivery publishes to subscribers on the target category.
"""

import asyncio

import pytest
from policy_broker.models import CheckRequest


def _gate_event_schedule(broker, category: str) -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_TIMER",
            path=category,
            method="event.schedule",
            principal="lambda-server",
            provenance="task-execution",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


class TestEventScheduleIntegration:
    """End-to-end: gate-check → (simulated) timer fire → subscriber receives."""

    def test_allow_schedule_with_cap_timer(
        self,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "external",
                    "method": "event.schedule",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_TIMER"],
                },
            ]
        )

        gate = _gate_event_schedule(policy_broker, "external")
        assert gate["allowed"], f"Broker DENY on event.schedule: {gate['response'].message}"

    def test_deny_schedule_without_policy(self, policy_broker):
        gate = _gate_event_schedule(policy_broker, "external")
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

    def test_deny_schedule_for_restricted_category(
        self,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "external",
                    "method": "event.schedule",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_TIMER"],
                },
            ]
        )

        gate = _gate_event_schedule(policy_broker, "system-shutdown")
        assert not gate["allowed"]

    @pytest.mark.asyncio
    async def test_scheduled_timer_fire_delivers_to_subscriber(
        self,
        event_bus,
        policy_broker,
        register_policy,
    ):
        """Simulate scheduler firing by publishing the timer payload after gate allow."""
        register_policy(
            [
                {
                    "path": "external",
                    "method": "event.schedule",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_TIMER"],
                },
                {
                    "path": "external",
                    "method": "event.publish",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_EVENT_PUBLISH"],
                },
            ]
        )

        gate = _gate_event_schedule(policy_broker, "external")
        assert gate["allowed"]

        received = []

        async def handler(payload):
            received.append(payload)

        event_bus.subscribers["external"] = [handler]

        timer_payload = {
            "task_id": "sched-001",
            "source": "scheduler",
            "pattern": "timer.fire",
        }
        # Rust event-bus injects scheduled events via the same publish pipeline.
        event_bus.publish("external", timer_payload)
        await asyncio.sleep(0.05)

        assert len(received) == 1
        assert received[0]["task_id"] == "sched-001"
        assert received[0]["source"] == "scheduler"
