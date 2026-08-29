"""RocksDB backend tests — skipped when python-rocksdb is not installed."""

import pytest

rocksdb = pytest.importorskip("rocksdb", reason="python-rocksdb not installed")

import tempfile  # noqa: E402
import os  # noqa: E402
from state_store.rocksdb_backend import RocksDBBackend  # noqa: E402
from state_store.models import PatchOp, PatchOpType  # noqa: E402


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
