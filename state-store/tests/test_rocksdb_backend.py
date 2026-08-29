import pytest
import tempfile
import os
from state_store.rocksdb_backend import RocksDBBackend
from state_store.models import PatchOp, PatchOpType


@pytest.fixture
def db():
    with tempfile.TemporaryDirectory() as temp_dir:
        db_path = os.path.join(temp_dir, "test_db")
        yield RocksDBBackend(db_path)


def test_state_put_and_get(db):
    db.put("test.key", "value")
    result = db.get("test.key")
    assert result.value == "value"


def test_state_delete(db):
    db.put("test.key", "value")
    db.delete("test.key")
    result = db.get("test.key")
    assert result is None


def test_state_patch(db):
    ops = [
        PatchOp(path="test.key", op=PatchOpType.SET, value="value"),
        PatchOp(path="test.counter", op=PatchOpType.INCREMENT, value=1),
    ]
    results = db.patch(ops)
    assert results["test.key"][1] == "value"
    assert results["test.counter"][1] == 1


def test_state_snapshot(db):
    db.put("test.key", "value")
    db.create_snapshot()
    db.put("test.key", "new_value")
    snapshot_value = db.get_snapshot("test.key")
    current_value = db.get("test.key")
    assert snapshot_value.value == "value"
    assert current_value.value == "new_value"
    db.release_snapshot()