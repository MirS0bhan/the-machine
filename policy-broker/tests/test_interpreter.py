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
        capabilities=["CAP_IPC_CALL"]
    )
    policy_doc = PolicyDoc(rules=[rule])
    interpreter.register(policy_doc)

    request = CheckRequest(
        method="lambda.register",
        request={"name": "calc.multiply"},
        provenance="agent"
    )
    response = interpreter.check(request)
    assert response.decision == "ALLOW"


def test_deny_rule(interpreter: PolicyInterpreter):
    rule = Rule(
        path="state.*",
        method="*",
        decision="DENY",
        capabilities=["CAP_STATE_WRITE"]
    )
    policy_doc = PolicyDoc(rules=[rule])
    interpreter.register(policy_doc)

    request = CheckRequest(
        method="state.set",
        request={"key": "ui.theme", "value": "dark"},
        provenance="agent"
    )
    response = interpreter.check(request)
    assert response.decision == "DENY"


def test_first_match_wins(interpreter: PolicyInterpreter):
    allow_rule = Rule(
        path="lambda.*",
        method="*",
        decision="ALLOW",
        capabilities=["CAP_IPC_CALL"]
    )
    deny_rule = Rule(
        path="lambda.*",
        method="lambda.register",
        decision="DENY",
        capabilities=[]
    )
    policy_doc = PolicyDoc(rules=[allow_rule, deny_rule])
    interpreter.register(policy_doc)

    request = CheckRequest(
        method="lambda.register",
        request={"name": "calc.multiply"},
        provenance="agent"
    )
    response = interpreter.check(request)
    assert response.decision == "ALLOW"


def test_default_deny(interpreter: PolicyInterpreter):
    request = CheckRequest(
        method="unknown.method",
        request={},
        provenance="agent"
    )
    response = interpreter.check(request)
    assert response.decision == "DENY"