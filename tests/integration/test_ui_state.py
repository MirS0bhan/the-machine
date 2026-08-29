"""
Cross-component integration tests: UI Runtime → State Store (policy-gated).

Workflow:
    1. UI Runtime patches a form field via `state.patch`.
    2. Policy Broker gates the call using `CAP_STATE_WRITE`.
    3. State Store reflects the update.
    4. A subscriber on `state.watch` receives the change.

Assertions:
    - Policy Broker allows `state.patch` for `ui.task.*` paths.
    - State Store reflects the update.
    - `state.watch` streams the change to subscribers.
"""

import asyncio
import pytest
from policy_broker.models import CheckRequest


# ── helpers ────────────────────────────────────────────────────────────

def _gate_state_write(broker, path: str) -> dict:
    resp = broker.check(CheckRequest(
        capability="CAP_STATE_WRITE",
        path=path,
        method="state.patch",
        principal="ui-runtime",
        provenance="user-input",
    ))
    return {"allowed": resp.decision == "ALLOW", "response": resp}


def _gate_state_read(broker, path: str) -> dict:
    resp = broker.check(CheckRequest(
        capability="CAP_STATE_READ",
        path=path,
        method="state.watch",
        principal="ui-runtime",
        provenance="user-input",
    ))
    return {"allowed": resp.decision == "ALLOW", "response": resp}


# ── tests ──────────────────────────────────────────────────────────────


class TestUIStatePolicyGated:
    """End-to-end: patch → gate-check → read-back → watch → assert."""

    def test_patch_form_field_and_read_back(
        self,
        state_store,
        policy_broker,
        register_policy,
    ):
        register_policy([
            {
                "path": "ui.task.*",
                "method": "state.patch",
                "decision": "ALLOW",
                "capabilities": ["CAP_STATE_WRITE"],
            },
            {
                "path": "ui.task.*",
                "method": "state.watch",
                "decision": "ALLOW",
                "capabilities": ["CAP_STATE_READ"],
            },
        ])

        path = "ui.task.form.name"
        gate = _gate_state_write(policy_broker, path)
        assert gate["allowed"], f"Broker DENY on patch: {gate['response'].message}"

        state_store.put(path, "Alice")
        stored = state_store.get(path)
        assert stored is not None
        assert stored["value"] == "Alice"

    def test_patch_multiple_fields(
        self,
        state_store,
        policy_broker,
        register_policy,
    ):
        register_policy([
            {
                "path": "ui.task.*",
                "method": "state.patch",
                "decision": "ALLOW",
                "capabilities": ["CAP_STATE_WRITE"],
            },
        ])

        ops = [
            {"path": "ui.task.form.first_name", "value": "Ada"},
            {"path": "ui.task.form.last_name", "value": "Lovelace"},
            {"path": "ui.task.form.email", "value": "ada@example.com"},
        ]

        for op in ops:
            gate = _gate_state_write(policy_broker, op["path"])
            assert gate["allowed"]

        state_store.patch(ops)

        assert state_store.get("ui.task.form.first_name")["value"] == "Ada"
        assert state_store.get("ui.task.form.last_name")["value"] == "Lovelace"
        assert state_store.get("ui.task.form.email")["value"] == "ada@example.com"

    def test_deny_patch_outside_ui_task_pattern(
        self,
        policy_broker,
        register_policy,
    ):
        register_policy([
            {
                "path": "ui.task.*",
                "method": "state.patch",
                "decision": "ALLOW",
                "capabilities": ["CAP_STATE_WRITE"],
            },
        ])

        gate = _gate_state_write(policy_broker, "system.kernel.params")
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

    def test_watch_streams_change_to_subscriber(
        self,
        state_store,
        policy_broker,
        register_policy,
    ):
        register_policy([
            {
                "path": "ui.task.*",
                "method": "state.patch",
                "decision": "ALLOW",
                "capabilities": ["CAP_STATE_WRITE"],
            },
            {
                "path": "ui.task.*",
                "method": "state.watch",
                "decision": "ALLOW",
                "capabilities": ["CAP_STATE_READ"],
            },
        ])

        path = "ui.task.status"
        gate = _gate_state_read(policy_broker, path)
        assert gate["allowed"], f"Broker DENY on watch: {gate['response'].message}"

        queue = state_store.watch(path)

        state_store.put(path, "running")
        item = queue.get_nowait()
        assert item["value"] == "running"
        assert item["path"] == path
        assert item["revision"] >= 1

    def test_watch_receives_sequence_of_changes(
        self,
        state_store,
    ):
        path = "ui.task.progress"
        queue = state_store.watch(path)

        for pct in (10, 25, 50, 75, 100):
            state_store.put(path, pct)

        received = []
        while not queue.empty():
            received.append(queue.get_nowait())

        values = [r["value"] for r in received]
        assert values == [10, 25, 50, 75, 100]

    def test_revision_monotonically_increases(
        self,
        state_store,
    ):
        path = "ui.task.counter"
        revs = []
        for i in range(5):
            result = state_store.put(path, i)
            revs.append(result["revision"])

        assert revs == sorted(revs)
        assert len(set(revs)) == 5, "All revisions must be unique"

    def test_patch_without_policy_denied(
        self,
        state_store,
        policy_broker,
    ):
        """No policy registered → default deny."""
        gate = _gate_state_write(policy_broker, "ui.task.anything")
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"
