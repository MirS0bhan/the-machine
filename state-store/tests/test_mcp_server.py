"""MCP server route tests via backend (no HTTP TestClient dependency)."""

import pytest
from state_store.memory_backend import MemoryBackend
from state_store.models import PatchOp, PatchOpType


@pytest.fixture
def db():
    return MemoryBackend()


def test_mcp_state_get_flow(db: MemoryBackend):
    db.patch([PatchOp(path="test.key", op=PatchOpType.SET, value="value")])
    result = db.get("test.key")
    assert result is not None
    assert result.value == "value"


def test_mcp_state_patch_increment(db: MemoryBackend):
    results = db.patch([PatchOp(path="test.counter", op=PatchOpType.INCREMENT, value=1)])
    assert results["test.counter"][1] == 1


def test_mcp_state_watch_revision_increases(db: MemoryBackend):
    db.patch([PatchOp(path="test.watch.key", op=PatchOpType.SET, value="initial")])
    first = db.get("test.watch.key")
    db.patch([PatchOp(path="test.watch.key", op=PatchOpType.SET, value="updated")])
    second = db.get("test.watch.key")
    assert first is not None and second is not None
    assert second.revision > first.revision
    assert second.value == "updated"
