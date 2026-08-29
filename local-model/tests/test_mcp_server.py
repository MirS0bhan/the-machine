# local-model/tests/test_mcp_server.py
import pytest
from fastapi.testclient import TestClient
from local_model.mcp_server import app
from local_model.models import CompletionRequest, EmbeddingRequest

client = TestClient(app)


def test_complete_endpoint():
    request = {"prompt": "Hello, world!", "max_tokens": 10, "privacy_tags": ["CAP_MIC"]}
    response = client.post("/mcp/localmodel.complete", json=request)
    assert response.status_code == 200
    assert "text" in response.json()
    assert response.json()["privacy_tag"] == "CAP_MIC"


def test_embed_endpoint():
    request = {"text": "Hello, world!", "privacy_tags": ["CAP_FS_READ"]}
    response = client.post("/mcp/localmodel.embed", json=request)
    assert response.status_code == 200
    assert "embedding" in response.json()
    assert response.json()["privacy_tag"] == "CAP_FS_READ"


def test_health_endpoint():
    response = client.get("/mcp/localmodel.health")
    assert response.status_code == 200
    assert response.json()["status"] == "healthy"