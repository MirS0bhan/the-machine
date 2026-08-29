from datetime import datetime
from typing import List, Dict, Optional
from policy_broker.models import AuditEntry, DecisionType


class AuditLogger:
    """
    Immutable audit log for all policy.check calls.
    """

    def __init__(self, state_store_client):
        self.state_store = state_store_client

    def log(self, method: str, request: Dict, provenance: str, decision: DecisionType, correlation_id: Optional[str] = None) -> AuditEntry:
        """Log a policy check decision."""
        entry = AuditEntry(
            timestamp=datetime.utcnow(),
            method=method,
            request=request,
            provenance=provenance,
            decision=decision,
            correlation_id=correlation_id
        )
        self.state_store.append_audit_log(entry)
        return entry

    def query(self, filter: Dict) -> List[AuditEntry]:
        """Query the audit log with a filter."""
        return self.state_store.query_audit_log(filter)