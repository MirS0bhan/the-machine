# local-model/tests/test_engine.py
import pytest
from local_model.engine import LocalModelEngine
from local_model.models import CompletionRequest, EmbeddingRequest


@pytest.fixture
def engine():
    # Use a small test model for unit tests
    return LocalModelEngine(model_path="/models/phi-3-q4.gguf")


def test_complete(engine):
    request = CompletionRequest(
        prompt="Hello, world!",
        max_tokens=10,
        privacy_tags=["CAP_MIC"],
    )
    response = engine.complete(request)
    assert isinstance(response.text, str)
    assert response.privacy_tag == "CAP_MIC"


def test_embed(engine):
    request = EmbeddingRequest(text="Hello, world!", privacy_tags=["CAP_FS_READ"])
    response = engine.embed(request)
    assert isinstance(response.embedding, list)
    assert len(response.embedding) > 0
    assert response.privacy_tag == "CAP_FS_READ"


def test_privacy_tag_none(engine):
    request = CompletionRequest(prompt="Hello, world!", max_tokens=10)
    response = engine.complete(request)
    assert response.privacy_tag is None