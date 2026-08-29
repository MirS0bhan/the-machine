# local-model/tests/test_mcp_server.py
"""MCP server tests via engine (no HTTP TestClient dependency)."""

import pytest
from local_model.engine import LocalModelEngine
from local_model.models import CompletionRequest, EmbeddingRequest, IntentRequest
from local_model.health import get_health_status


@pytest.fixture
def engine():
    return LocalModelEngine(model_path="/nonexistent/model.gguf")


def test_complete_endpoint(engine):
    response = engine.complete(CompletionRequest(
        prompt="Hello, world!", max_tokens=10, privacy_tags=["CAP_MIC"]
    ))
    assert isinstance(response.text, str)
    assert response.privacy_tag == "CAP_MIC"


def test_embed_endpoint(engine):
    response = engine.embed(EmbeddingRequest(
        text="Hello, world!", privacy_tags=["CAP_FS_READ"]
    ))
    assert isinstance(response.embedding, list)
    assert len(response.embedding) > 0
    assert response.privacy_tag == "CAP_FS_READ"


def test_classify_intent(engine):
    response = engine.classify_intent(IntentRequest(text="play a video"))
    assert response.intent == "media.play"


def test_health_endpoint():
    status = get_health_status()
    assert status.status in ("healthy", "degraded", "unavailable")
