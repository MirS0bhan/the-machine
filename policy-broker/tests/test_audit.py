import pytest
from policy_broker.audit import AuditLogger
from policy_broker.models import AuditEntry
from policy_broker.state_store import StateStoreClient
from datetime import datetime


@pytest.fixture
def audit_logger():
    state_store = StateStoreClient()
    return AuditLogger(state_store)


def test_audit_log(audit_logger: AuditLogger):
    method = "lambda.register"
    request = {"name": "calc.multiply"}
    provenance = "agent"
    decision = "ALLOW"

    entry = audit_logger.log(method, request, provenance, decision)
    assert entry.method == method
    assert entry.request == request
    assert entry.provenance == provenance
    assert entry.decision == decision
    assert isinstance(entry.timestamp, datetime)


def test_audit_query(audit_logger: AuditLogger, mocker):
    mock_query = mocker.patch.object(audit_logger.state_store, "query_audit_log", return_value=[])
    filter = {"method": "lambda.*"}
    audit_logger.query(filter)
    mock_query.assert_called_once_with(filter)