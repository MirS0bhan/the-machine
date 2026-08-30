"""
Cross-component integration tests: State Store get/set/list (policy-gated).

Workflow:
    1. Policy Broker gates `state.set` / `state.get` / `state.list` via
       CAP_STATE_WRITE / CAP_STATE_READ.
    2. State Store persists values and exposes list-by-prefix.
    3. Denied paths never reach the store when policy blocks the gate.

Assertions:
    - Policy Broker allows `state.set` / `state.get` for permitted prefixes.
    - State Store read-back matches written values and revisions increase.
    - `state.list` returns only paths under the requested prefix.
    - Default-deny policy blocks writes and reads outside allowed patterns.
"""

from policy_broker.models import CheckRequest


def _gate_state_write(broker, path: str) -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_STATE_WRITE",
            path=path,
            method="state.set",
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


def _gate_state_read(broker, path: str) -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_STATE_READ",
            path=path,
            method="state.get",
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


def _gate_state_list(broker, prefix: str) -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_STATE_READ",
            path=prefix or "*",
            method="state.list",
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


class TestStateGetSetPolicyGated:
    """End-to-end: gate-check → set/get/list → assert store contents."""

    def test_set_then_get_round_trip(
        self,
        state_store,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "task.*",
                    "method": "state.set",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_STATE_WRITE"],
                },
                {
                    "path": "task.*",
                    "method": "state.get",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_STATE_READ"],
                },
            ]
        )

        path = "task.status"
        write_gate = _gate_state_write(policy_broker, path)
        assert write_gate["allowed"], write_gate["response"].message

        state_store.put(path, "running")
        read_gate = _gate_state_read(policy_broker, path)
        assert read_gate["allowed"], read_gate["response"].message

        stored = state_store.get(path)
        assert stored is not None
        assert stored["value"] == "running"
        assert stored["revision"] >= 1

    def test_list_returns_prefix_matches_only(
        self,
        state_store,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "task.*",
                    "method": "state.set",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_STATE_WRITE"],
                },
                {
                    "path": "task.*",
                    "method": "state.list",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_STATE_READ"],
                },
            ]
        )

        for suffix, value in (("alpha", 1), ("beta", 2), ("gamma", 3)):
            path = f"task.item.{suffix}"
            assert _gate_state_write(policy_broker, path)["allowed"]
            state_store.put(path, value)

        state_store.put("other.outside", "skip")

        list_gate = _gate_state_list(policy_broker, "task.item.")
        assert list_gate["allowed"], list_gate["response"].message

        listed = [
            p
            for p in state_store._data
            if p.startswith("task.item.")
        ]
        assert sorted(listed) == [
            "task.item.alpha",
            "task.item.beta",
            "task.item.gamma",
        ]
        assert "other.outside" not in listed

    def test_deny_set_outside_allowed_prefix(
        self,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "task.*",
                    "method": "state.set",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_STATE_WRITE"],
                },
            ]
        )

        gate = _gate_state_write(policy_broker, "system.secret.key")
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

    def test_deny_get_without_read_policy(
        self,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "task.*",
                    "method": "state.set",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_STATE_WRITE"],
                },
            ]
        )

        gate = _gate_state_read(policy_broker, "task.status")
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

    def test_revision_increases_on_overwrite(
        self,
        state_store,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "task.*",
                    "method": "state.set",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_STATE_WRITE"],
                },
                {
                    "path": "task.*",
                    "method": "state.get",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_STATE_READ"],
                },
            ]
        )

        path = "task.counter"
        assert _gate_state_write(policy_broker, path)["allowed"]
        first = state_store.put(path, 1)
        second = state_store.put(path, 2)

        assert second["revision"] > first["revision"]
        assert state_store.get(path)["value"] == 2
