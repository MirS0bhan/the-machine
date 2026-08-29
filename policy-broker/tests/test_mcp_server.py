import pytest
from fastapi.testclient import TestClient
from policy_broker.mcp_server import app
from policy_broker.models import PolicyDoc, Rule, CheckRequest

client = TestClient(app)


def test_policy_check_allow():
    rule = Rule(
        path="lambda.*",
        method="*",
        decision="ALLOW",
        capabilities=["CAP_IPC_CALL"]
    )
    policy_doc = PolicyDoc(rules=[rule])
    client.post("/policy/register", json=policy_doc.model_dump())

    request = CheckRequest(
        method="lambda.register",
        request={"name": "calc.multiply"},
        provenance="agent"
    )
    response = client.post("/policy/check", json=request.model_dump())
    assert response.status_code == 200
    assert response.json()["decision"] == "ALLOW"


def test_policy_check_deny():
    request = CheckRequest(
        method="unknown.method",
        request={},
        provenance="agent"
    )
    response = client.post("/policy/check", json=request.model_dump())
    assert response.status_code == 200
    assert response.json()["decision"] == "DENY"


def test_policy_register():
    rule = Rule(
        path="state.*",
        method="*",
        decision="DENY",
        capabilities=["CAP_STATE_WRITE"]
    )
    policy_doc = PolicyDoc(rules=[rule])
    response = client.post("/policy/register", json=policy_doc.model_dump())
    assert response.status_code == 204