"""
Cross-component integration tests: Agent Core → Local Model (policy-gated).

Workflow:
    1. Agent Core requests inference via `localmodel.complete`.
    2. Policy Broker gates the call using `CAP_IPC_CALL`.
    3. Local Model engine returns a completion (stub when no GGUF is present).

Assertions:
    - Policy Broker allows `localmodel.complete` for agent-core when permitted.
    - Local Model returns text and preserves privacy tags on the response.
    - Policy Broker denies calls when no matching rule exists.
"""

import sys
from pathlib import Path

import pytest
from policy_broker.models import CheckRequest

_LOCAL_MODEL_DIR = Path(__file__).resolve().parents[2] / "local-model"
if str(_LOCAL_MODEL_DIR) not in sys.path:
    sys.path.insert(0, str(_LOCAL_MODEL_DIR))

from local_model.engine import LocalModelEngine  # noqa: E402
from local_model.models import CompletionRequest  # noqa: E402


def _gate_localmodel(broker, method: str = "localmodel.complete") -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_IPC_CALL",
            path="localmodel.complete",
            method=method,
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


@pytest.fixture
def local_model_engine() -> LocalModelEngine:
    return LocalModelEngine(model_path="/nonexistent/model.gguf")


class TestLocalModelCompleteIntegration:
    def test_agent_core_complete_allowed_and_returns_text(
        self,
        policy_broker,
        register_policy,
        local_model_engine,
    ):
        register_policy(
            [
                {
                    "path": "localmodel.*",
                    "method": "localmodel.complete",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
            ]
        )

        gate = _gate_localmodel(policy_broker)
        assert gate["allowed"], f"Broker DENY: {gate['response'].message}"

        response = local_model_engine.complete(
            CompletionRequest(
                prompt="Plan a calculator widget",
                max_tokens=32,
                privacy_tags=["CAP_MIC"],
            )
        )
        assert response.text.startswith("[stub]")
        assert response.privacy_tag == "CAP_MIC"

    def test_complete_denied_without_policy(self, policy_broker, local_model_engine):
        gate = _gate_localmodel(policy_broker)
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

        # Engine still works in-process; production path is broker-gated first.
        response = local_model_engine.complete(
            CompletionRequest(prompt="hello", max_tokens=8)
        )
        assert isinstance(response.text, str)
