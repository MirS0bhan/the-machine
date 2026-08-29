"""MCP server endpoint tests (via interpreter, no HTTP client dependency)."""

import pytest
from policy_broker.interpreter import PolicyInterpreter
from policy_broker.models import PolicyDoc, Rule, CheckRequest
from policy_broker.state_store import StateStoreClient


@pytest.fixture
def interpreter():
    return PolicyInterpreter(StateStoreClient())


def test_policy_check_allow(interpreter: PolicyInterpreter):
    rule = Rule(
        path="lambda.*",
        method="*",
        decision="ALLOW",
        capabilities=["CAP_IPC_CALL"],
    )
    interpreter.register(PolicyDoc(rules=[rule]))

    response = interpreter.check(CheckRequest(
        capability="CAP_IPC_CALL",
        path="lambda.register",
        method="lambda.register",
        principal="agent-core",
        provenance="agent",
    ))
    assert response.decision == "ALLOW"


def test_policy_check_deny(interpreter: PolicyInterpreter):
    response = interpreter.check(CheckRequest(
        capability="CAP_UNKNOWN",
        path="unknown.method",
        method="unknown.method",
        principal="agent-core",
        provenance="agent",
    ))
    assert response.decision == "DENY"


def test_policy_register(interpreter: PolicyInterpreter):
    rule = Rule(
        path="state.*",
        method="*",
        decision="DENY",
        capabilities=["CAP_STATE_WRITE"],
    )
    interpreter.register(PolicyDoc(rules=[rule]))

    response = interpreter.check(CheckRequest(
        capability="CAP_STATE_WRITE",
        path="state.set",
        method="state.set",
        principal="agent-core",
        provenance="agent",
    ))
    assert response.decision == "DENY"
