"""State Store backend tests (memory backend — no RocksDB required)."""

import pytest
from state_store.memory_backend import MemoryBackend
from state_store.models import PatchOp, PatchOpType


@pytest.fixture
def db():
    return MemoryBackend("/tmp/test-state-store")


def test_state_put_and_get(db: MemoryBackend):
    db.put("test.key", "value")
    result = db.get("test.key")
    assert result is not None
    assert result.value == "value"


def test_state_delete(db: MemoryBackend):
    db.put("test.key", "value")
    assert db.delete("test.key")
    assert db.get("test.key") is None


def test_state_patch_set_and_increment(db: MemoryBackend):
    ops = [
        PatchOp(path="test.key", op=PatchOpType.SET, value="value"),
        PatchOp(path="test.counter", op=PatchOpType.INCREMENT, value=1),
    ]
    results = db.patch(ops)
    assert results["test.key"][1] == "value"
    assert results["test.counter"][1] == 1


def test_state_patch_increment_existing(db: MemoryBackend):
    db.put("test.counter", 5)
    results = db.patch([PatchOp(path="test.counter", op=PatchOpType.INCREMENT, value=3)])
    assert results["test.counter"][1] == 8


def test_list_paths(db: MemoryBackend):
    db.put("ui.task.name", "Alice")
    db.put("ui.task.age", 30)
    db.put("system.boot", True)
    paths = db.list_paths("ui.task.")
    assert sorted(paths) == ["ui.task.age", "ui.task.name"]
