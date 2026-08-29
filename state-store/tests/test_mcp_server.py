import pytest
from fastapi.testclient import TestClient
from state_store.mcp_server import app
from state_store.models import PatchOp, PatchOpType


client = TestClient(app)


def test_mcp_state_get():
    # Setup
    ops = [PatchOp(path="test.key", op=PatchOpType.SET, value="value")]
    client.post("/state.patch", json=[op.model_dump() for op in ops])

    # Test
    response = client.get("/state.get?path=test.key")
    assert response.status_code == 200
    assert response.json()["value"] == "value"


def test_mcp_state_patch():
    ops = [PatchOp(path="test.counter", op=PatchOpType.INCREMENT, value=1)]
    response = client.post("/state.patch", json=[op.model_dump() for op in ops])
    assert response.status_code == 200
    assert response.json()["results"]["test.counter"][1] == 1


@pytest.mark.asyncio
async def test_mcp_state_watch():
    # Setup
    ops = [PatchOp(path="test.watch.key", op=PatchOpType.SET, value="initial")]
    client.post("/state.patch", json=[op.model_dump() for op in ops])

    # Test SSE stream (simplified)
    response = client.get("/state.watch?path_prefix=test.watch.key")
    assert response.status_code == 200
    assert response.headers["content-type"] == "text/event-stream"