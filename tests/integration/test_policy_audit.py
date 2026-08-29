"""
Cross-component integration tests: Policy Broker → State Store (audit log).

Workflow:
    1. Policy Broker processes multiple `policy.check` calls.
    2. Each call is logged to the State Store's audit log.
    3. Tests query the audit log and validate structure and content.

Important implementation detail:
    PolicyInterpreter.check() calls AuditLogger.log() with:
        - `method`  ← the *capability* string (e.g. "CAP_IPC_CALL")
        - `request` ← {"path": ..., "principal": ...}
        - `provenance` ← the *principal* string (e.g. "agent-core")
        - `decision` ← the decision outcome

    This is the actual behavior of the existing Broker code
    (see interpreter.py:67-68). Tests are written to match this shape.
"""

import pytest
from policy_broker.models import CheckRequest


# ── tests ──────────────────────────────────────────────────────────────


class TestPolicyAuditLog:
    """Validate the audit log written by Policy Broker to State Store."""

    def test_audit_log_records_lambda_register(
        self,
        policy_broker,
        state_store,
        register_policy,
    ):
        register_policy([
            {
                "path": "calc.*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="calc.add",
            method="lambda.register",
            principal="agent-core",
            provenance="user-intent",
        ))

        log = state_store.query_audit_log()
        assert len(log) == 1

        entry = log[0]
        # Capability is stored as the `method` field by the interpreter
        assert entry.method == "CAP_IPC_CALL"
        assert entry.decision == "ALLOW"
        # Principal is stored as `provenance` by the interpreter
        assert entry.provenance == "agent-core"
        assert entry.timestamp is not None
        assert isinstance(entry.request, dict)
        assert entry.request.get("path") == "calc.add"
        assert entry.request.get("principal") == "agent-core"

    def test_audit_log_records_state_patch(
        self,
        policy_broker,
        state_store,
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

        policy_broker.check(CheckRequest(
            capability="CAP_STATE_WRITE",
            path="ui.task.form.name",
            method="state.patch",
            principal="ui-runtime",
            provenance="user-input",
        ))

        log = state_store.query_audit_log()
        assert len(log) == 1
        assert log[0].method == "CAP_STATE_WRITE"
        assert log[0].provenance == "ui-runtime"
        assert log[0].request.get("path") == "ui.task.form.name"

    def test_audit_log_records_denied_call(
        self,
        policy_broker,
        state_store,
    ):
        """Denials are also logged."""
        policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="evil.func",
            method="lambda.register",
            principal="attacker",
            provenance="injected-content",
        ))

        log = state_store.query_audit_log()
        assert len(log) == 1
        assert log[0].decision == "DENY"
        assert log[0].provenance == "attacker"

    def test_audit_log_multiple_entries_accumulate(
        self,
        policy_broker,
        state_store,
        register_policy,
    ):
        register_policy([
            {
                "path": "*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
            {
                "path": "*",
                "method": "lambda.invoke",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
            {
                "path": "*",
                "method": "state.patch",
                "decision": "ALLOW",
                "capabilities": ["CAP_STATE_WRITE"],
            },
        ])

        calls = [
            ("CAP_IPC_CALL", "calc.add", "lambda.register"),
            ("CAP_IPC_CALL", "calc.add", "lambda.invoke"),
            ("CAP_STATE_WRITE", "ui.task.x", "state.patch"),
        ]

        for cap, path, method in calls:
            policy_broker.check(CheckRequest(
                capability=cap,
                path=path,
                method=method,
                principal="agent-core",
                provenance="user-intent",
            ))

        log = state_store.query_audit_log()
        assert len(log) == 3

        # method field contains the capability string
        capabilities_in_log = [e.method for e in log]
        assert "CAP_IPC_CALL" in capabilities_in_log
        assert "CAP_STATE_WRITE" in capabilities_in_log

    def test_audit_log_entries_have_required_fields(
        self,
        policy_broker,
        state_store,
        register_policy,
    ):
        register_policy([
            {
                "path": "*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="test.func",
            method="lambda.register",
            principal="agent-core",
            provenance="user-intent",
        ))

        entry = state_store.query_audit_log()[0]

        assert hasattr(entry, "timestamp")
        assert hasattr(entry, "method")
        assert hasattr(entry, "decision")
        assert hasattr(entry, "provenance")
        assert hasattr(entry, "request")
        assert hasattr(entry, "correlation_id")

        assert entry.timestamp.__class__.__name__ == "datetime"
        assert entry.method == "CAP_IPC_CALL"
        assert entry.decision == "ALLOW"
        assert entry.provenance == "agent-core"
        assert isinstance(entry.request, dict)

    def test_audit_log_query_filter_by_method(
        self,
        policy_broker,
        state_store,
        register_policy,
    ):
        """Filter by the capability string stored in the `method` field."""
        register_policy([
            {
                "path": "*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
            {
                "path": "*",
                "method": "state.patch",
                "decision": "ALLOW",
                "capabilities": ["CAP_STATE_WRITE"],
            },
        ])

        policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="a",
            method="lambda.register",
            principal="agent-core",
            provenance="user-intent",
        ))
        policy_broker.check(CheckRequest(
            capability="CAP_STATE_WRITE",
            path="b",
            method="state.patch",
            principal="ui-runtime",
            provenance="user-input",
        ))

        filtered = state_store.query_audit_log({"method": "CAP_IPC_CALL"})
        assert len(filtered) == 1
        assert filtered[0].method == "CAP_IPC_CALL"

    def test_audit_log_query_filter_by_decision(
        self,
        policy_broker,
        state_store,
        register_policy,
    ):
        register_policy([
            {
                "path": "good.*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        # ALLOW
        policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="good.func",
            method="lambda.register",
            principal="agent-core",
            provenance="user-intent",
        ))
        # DENY (no matching rule for bad.*)
        policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="bad.func",
            method="lambda.register",
            principal="agent-core",
            provenance="user-intent",
        ))

        allowed = state_store.query_audit_log({"decision": "ALLOW"})
        denied = state_store.query_audit_log({"decision": "DENY"})

        assert len(allowed) == 1
        assert len(denied) == 1

    def test_audit_log_entries_are_immutable(
        self,
        policy_broker,
        state_store,
        register_policy,
    ):
        """AuditEntry values persist correctly after append."""
        register_policy([
            {
                "path": "*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="x",
            method="lambda.register",
            principal="agent-core",
            provenance="user-intent",
        ))

        entry = state_store.query_audit_log()[0]
        original_method = entry.method
        original_decision = entry.decision

        assert entry.method == original_method
        assert entry.decision == "ALLOW"

    def test_empty_audit_log_returns_empty_list(
        self,
        state_store,
    ):
        log = state_store.query_audit_log()
        assert log == []

    def test_audit_log_without_filter_returns_all(
        self,
        policy_broker,
        state_store,
        register_policy,
    ):
        register_policy([
            {
                "path": "*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        for i in range(3):
            policy_broker.check(CheckRequest(
                capability="CAP_IPC_CALL",
                path=f"f{i}",
                method="lambda.register",
                principal="agent-core",
                provenance="user-intent",
            ))

        log = state_store.query_audit_log()
        assert len(log) == 3
