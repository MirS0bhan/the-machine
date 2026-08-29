import json
import os
import rocksdb
from typing import Any, Dict, List, Optional, Tuple
from .models import PatchOp, PatchOpType, StateResponse


class RocksDBBackend:
    def __init__(self, db_path: str):
        """Initialize RocksDB backend."""
        self.db_path = db_path
        opts = rocksdb.Options()
        opts.create_if_missing = True
        opts.WAL_ttl_seconds = 0  # Disable WAL TTL for durability
        opts.WAL_size_limit_MB = 0  # Disable WAL size limit
        self.db = rocksdb.DB(db_path, opts)
        self.snapshot = None

    def get(self, path: str) -> Optional[StateResponse]:
        """Retrieve the value at `path`."""
        value = self.db.get(path.encode())
        if value is None:
            return None
        return StateResponse(**json.loads(value.decode()))

    def put(self, path: str, value: Any, revision: Optional[int] = None) -> int:
        """Put a value at `path` and return the new revision."""
        if revision is None:
            current = self.get(path)
            revision = current.revision + 1 if current else 0

        state = StateResponse(value=value, revision=revision)
        self.db.put(path.encode(), json.dumps(state.model_dump()).encode())
        self.db.flush(wal=True)  # fsync WAL
        return revision

    def delete(self, path: str) -> bool:
        """Delete the value at `path`."""
        if self.db.get(path.encode()) is None:
            return False
        self.db.delete(path.encode())
        self.db.flush(wal=True)  # fsync WAL
        return True

    def patch(self, ops: List[PatchOp]) -> Dict[str, Tuple[int, Any]]:
        """Apply a list of patch operations atomically."""
        batch = rocksdb.WriteBatch()
        results = {}

        for op in ops:
            current = self.get(op.path)
            revision = current.revision + 1 if current else 0
            new_value = self._apply_op(op, current.value if current else None)

            state = StateResponse(value=new_value, revision=revision)
            batch.put(op.path.encode(), json.dumps(state.model_dump()).encode())
            results[op.path] = (revision, new_value)

        self.db.write(batch)
        self.db.flush(wal=True)  # fsync WAL
        return results

    def _apply_op(self, op: PatchOp, current_value: Any) -> Any:
        """Apply a single patch operation."""
        if op.op == PatchOpType.SET:
            return op.value
        elif op.op == PatchOpType.INCREMENT:
            return (current_value or 0) + op.value
        elif op.op == PatchOpType.DECREMENT:
            return (current_value or 0) - op.value
        elif op.op == PatchOpType.TOGGLE:
            return not current_value if isinstance(current_value, bool) else bool(op.value)
        else:
            raise ValueError(f"Unsupported op: {op.op}")

    def create_snapshot(self) -> None:
        """Create a snapshot for atomic reads."""
        self.snapshot = self.db.snapshot()

    def release_snapshot(self) -> None:
        """Release the current snapshot."""
        if self.snapshot:
            self.snapshot.release()
            self.snapshot = None

    def get_snapshot(self, path: str) -> Optional[StateResponse]:
        """Get a value using the current snapshot."""
        if not self.snapshot:
            return self.get(path)
        value = self.snapshot.get(path.encode())
        if value is None:
            return None
        return StateResponse(**json.loads(value.decode()))