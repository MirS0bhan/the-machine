"""
Cross-component integration tests: Agent Core → Local Model classify (policy-gated).

Workflow:
    1. Agent Core requests routing via `localmodel.classify_intent`.
    2. Policy Broker gates the call using `CAP_IPC_CALL`.
    3. Local Model engine returns a taxonomy intent (stub when no GGUF is present).

Assertions:
    - Policy Broker allows `localmodel.classify_intent` for agent-core when permitted.
    - Classifier maps media / calc / generic phrases to the documented intents.
    - Privacy tags from sensitive capabilities are stamped on the response.
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
from local_model.models import IntentRequest  # noqa: E402


def _gate_classify(broker, method: str = "localmodel.classify_intent") -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_IPC_CALL",
            path="localmodel.classify_intent",
            method=method,
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


@pytest.fixture
def local_model_engine() -> LocalModelEngine:
    return LocalModelEngine(model_path="/nonexistent/model.gguf")


class TestLocalModelClassifyIntentIntegration:
    def test_agent_core_classify_allowed_and_routes_intents(
        self,
        policy_broker,
        register_policy,
        local_model_engine,
    ):
        register_policy(
            [
                {
                    "path": "localmodel.*",
                    "method": "localmodel.classify_intent",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
            ]
        )

        gate = _gate_classify(policy_broker)
        assert gate["allowed"], f"Broker DENY: {gate['response'].message}"

        media = local_model_engine.classify_intent(
            IntentRequest(text="play a video", privacy_tags=["CAP_MIC"])
        )
        assert media.intent == "media.play"
        assert media.confidence >= 0.8
        assert media.privacy_tag == "CAP_MIC"

        calc = local_model_engine.classify_intent(IntentRequest(text="calc 2+2"))
        assert calc.intent == "calc.eval"
        assert calc.privacy_tag is None

        generic = local_model_engine.classify_intent(
            IntentRequest(text="what is the weather")
        )
        assert generic.intent == "general.query"

    def test_classify_denied_without_policy(self, policy_broker, local_model_engine):
        gate = _gate_classify(policy_broker)
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

        # Engine still works in-process; production path is broker-gated first.
        response = local_model_engine.classify_intent(
            IntentRequest(text="play a video")
        )
        assert response.intent == "media.play"
