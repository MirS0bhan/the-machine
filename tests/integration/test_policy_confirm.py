"""
Cross-component integration tests: Policy Broker CONFIRM/HOLD decisions.

Workflow:
    1. Register a policy rule with CONFIRM or HOLD for a sensitive capability.
    2. A component calls `policy.check` (simulated via PolicyInterpreter).
    3. Broker returns CONFIRM/HOLD with a correlation_id for human approval.
    4. Audit log records the pending decision and correlation_id.

Assertions:
    - CONFIRM/HOLD responses include a non-empty correlation_id.
    - ALLOW responses omit correlation_id.
    - Audit log stores decision and correlation_id for CONFIRM/HOLD paths.
"""

import pytest
from policy_broker.models import CheckRequest


class TestPolicyConfirmHold:
    """End-to-end: register CONFIRM/HOLD rule → check → audit log."""

    def test_confirm_returns_correlation_id_and_audits(
        self,
        policy_broker,
        state_store,
        register_policy,
    ):
        register_policy([
            {
                "path": "marketplace.*",
                "method": "marketplace.install",
                "decision": "CONFIRM",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        resp = policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="marketplace.install",
            method="marketplace.install",
            principal="agent-core",
            provenance="user-intent",
        ))

        assert resp.decision == "CONFIRM"
        assert resp.correlation_id
        assert "CAP_IPC_CALL" in resp.correlation_id

        log = state_store.query_audit_log()
        assert len(log) == 1
        assert log[0].decision == "CONFIRM"
        assert log[0].correlation_id == resp.correlation_id
        assert log[0].provenance == "agent-core"
        assert log[0].request.get("path") == "marketplace.install"

    def test_hold_returns_correlation_id_and_audits(
        self,
        policy_broker,
        state_store,
        register_policy,
    ):
        register_policy([
            {
                "path": "systemd.*",
                "method": "systemd.stop",
                "decision": "HOLD",
                "capabilities": ["CAP_SYSTEM_MUTATE"],
            },
        ])

        resp = policy_broker.check(CheckRequest(
            capability="CAP_SYSTEM_MUTATE",
            path="systemd.stop",
            method="systemd.stop",
            principal="ui-runtime",
            provenance="user-input",
        ))

        assert resp.decision == "HOLD"
        assert resp.correlation_id
        assert "CAP_SYSTEM_MUTATE" in resp.correlation_id

        log = state_store.query_audit_log()
        assert len(log) == 1
        assert log[0].decision == "HOLD"
        assert log[0].correlation_id == resp.correlation_id

    def test_allow_omits_correlation_id(
        self,
        policy_broker,
        state_store,
        register_policy,
    ):
        register_policy([
            {
                "path": "calc.*",
                "method": "lambda.invoke",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        resp = policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="calc.add",
            method="lambda.invoke",
            principal="agent-core",
            provenance="user-intent",
        ))

        assert resp.decision == "ALLOW"
        assert resp.correlation_id is None

        log = state_store.query_audit_log()
        assert len(log) == 1
        assert log[0].decision == "ALLOW"
        assert log[0].correlation_id is None

    def test_deny_omits_correlation_id(
        self,
        policy_broker,
        state_store,
    ):
        resp = policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="evil.backdoor",
            method="lambda.register",
            principal="attacker",
            provenance="injected-content",
        ))

        assert resp.decision == "DENY"
        assert resp.correlation_id is None

        log = state_store.query_audit_log()
        assert len(log) == 1
        assert log[0].decision == "DENY"
        assert log[0].correlation_id is None

    def test_confirm_decisions_are_queryable_by_decision_filter(
        self,
        policy_broker,
        state_store,
        register_policy,
    ):
        register_policy([
            {
                "path": "marketplace.*",
                "method": "marketplace.install",
                "decision": "CONFIRM",
                "capabilities": ["CAP_IPC_CALL"],
            },
            {
                "path": "calc.*",
                "method": "lambda.invoke",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="marketplace.install",
            method="marketplace.install",
            principal="agent-core",
            provenance="user-intent",
        ))
        policy_broker.check(CheckRequest(
            capability="CAP_IPC_CALL",
            path="calc.add",
            method="lambda.invoke",
            principal="agent-core",
            provenance="user-intent",
        ))

        confirm_entries = state_store.query_audit_log({"decision": "CONFIRM"})
        allow_entries = state_store.query_audit_log({"decision": "ALLOW"})

        assert len(confirm_entries) == 1
        assert len(allow_entries) == 1
        assert confirm_entries[0].correlation_id is not None
        assert allow_entries[0].correlation_id is None
