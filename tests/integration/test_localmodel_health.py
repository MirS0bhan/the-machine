"""
Cross-component integration tests: Agent Core → Local Model health (policy-gated).

Workflow:
    1. Agent Core polls readiness via `localmodel.health` before routing inference.
    2. Policy Broker gates the call using `CAP_IPC_CALL`.
    3. Local Model health module reports status/load for Event Bus health category.

Assertions:
    - Policy Broker allows `localmodel.health` for agent-core when permitted.
    - Health response includes status and load fields (stub backend on test hosts).
    - Policy Broker denies health checks when no matching rule exists.
"""

import sys
from pathlib import Path

from policy_broker.models import CheckRequest

_LOCAL_MODEL_DIR = Path(__file__).resolve().parents[2] / "local-model"
if str(_LOCAL_MODEL_DIR) not in sys.path:
    sys.path.insert(0, str(_LOCAL_MODEL_DIR))

from local_model.health import get_health_status  # noqa: E402


def _gate_localmodel_health(broker, method: str = "localmodel.health") -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_IPC_CALL",
            path="localmodel.health",
            method=method,
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


class TestLocalModelHealthIntegration:
    def test_health_allowed_and_reports_status(
        self,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "localmodel.*",
                    "method": "localmodel.health",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
            ]
        )

        gate = _gate_localmodel_health(policy_broker)
        assert gate["allowed"], f"Broker DENY: {gate['response'].message}"

        health = get_health_status()
        assert health.status == "healthy"
        assert 0.0 <= health.load <= 1.0

    def test_health_denied_without_policy(self, policy_broker):
        gate = _gate_localmodel_health(policy_broker)
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"
