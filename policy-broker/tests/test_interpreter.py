import pytest
from policy_broker.interpreter import PolicyInterpreter
from policy_broker.models import PolicyDoc, Rule, CheckRequest, CheckResponse
from policy_broker.state_store import StateStoreClient


@pytest.fixture
def interpreter():
    state_store = StateStoreClient()
    return PolicyInterpreter(state_store)


def test_allow_rule(interpreter: PolicyInterpreter):
    rule = Rule(
        path="lambda.*",
        method="*",
        decision="ALLOW",
        capabilities=["CAP_IPC_CALL"],
    )
    policy_doc = PolicyDoc(rules=[rule])
    interpreter.register(policy_doc)

    request = CheckRequest(
        capability="CAP_IPC_CALL",
        path="lambda.register",
        method="lambda.register",
        principal="agent-core",
        provenance="agent",
    )
    response = interpreter.check(request)
    assert response.decision == "ALLOW"


def test_deny_rule(interpreter: PolicyInterpreter):
    rule = Rule(
        path="state.*",
        method="*",
        decision="DENY",
        capabilities=["CAP_STATE_WRITE"],
    )
    policy_doc = PolicyDoc(rules=[rule])
    interpreter.register(policy_doc)

    request = CheckRequest(
        capability="CAP_STATE_WRITE",
        path="ui.theme",
        method="state.set",
        principal="agent-core",
        provenance="agent",
    )
    response = interpreter.check(request)
    assert response.decision == "DENY"


def test_first_match_wins(interpreter: PolicyInterpreter):
    allow_rule = Rule(
        path="lambda.*",
        method="*",
        decision="ALLOW",
        capabilities=["CAP_IPC_CALL"],
    )
    deny_rule = Rule(
        path="lambda.*",
        method="lambda.register",
        decision="DENY",
        capabilities=["CAP_IPC_CALL"],
    )
    policy_doc = PolicyDoc(rules=[allow_rule, deny_rule])
    interpreter.register(policy_doc)

    request = CheckRequest(
        capability="CAP_IPC_CALL",
        path="lambda.register",
        method="lambda.register",
        principal="agent-core",
        provenance="agent",
    )
    response = interpreter.check(request)
    assert response.decision == "ALLOW"


def test_default_deny(interpreter: PolicyInterpreter):
    request = CheckRequest(
        capability="CAP_UNKNOWN",
        path="unknown",
        method="unknown.method",
        principal="agent-core",
        provenance="agent",
    )
    response = interpreter.check(request)
    assert response.decision == "DENY"
