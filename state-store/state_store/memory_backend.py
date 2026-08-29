"""In-memory state backend used when RocksDB is unavailable."""

from __future__ import annotations

from typing import Any, Dict, List, Optional, Tuple

from .models import PatchOp, PatchOpType, StateResponse


class MemoryBackend:
    """Drop-in replacement for RocksDBBackend for dev/test environments."""

    def __init__(self, db_path: str = "/tmp/state-store"):
        self.db_path = db_path
        self._data: Dict[str, StateResponse] = {}

    def get(self, path: str) -> Optional[StateResponse]:
        return self._data.get(path)

    def put(self, path: str, value: Any, revision: Optional[int] = None) -> int:
        if revision is None:
            current = self.get(path)
            revision = current.revision + 1 if current else 0
        self._data[path] = StateResponse(value=value, revision=revision)
        return revision

    def delete(self, path: str) -> bool:
        if path not in self._data:
            return False
        del self._data[path]
        return True

    def patch(self, ops: List[PatchOp]) -> Dict[str, Tuple[int, Any]]:
        results: Dict[str, Tuple[int, Any]] = {}
        for op in ops:
            current = self.get(op.path)
            revision = current.revision + 1 if current else 0
            new_value = self._apply_op(op, current.value if current else None)
            self._data[op.path] = StateResponse(value=new_value, revision=revision)
            results[op.path] = (revision, new_value)
        return results

    def _apply_op(self, op: PatchOp, current_value: Any) -> Any:
        if op.op == PatchOpType.SET:
            return op.value
        if op.op == PatchOpType.INCREMENT:
            return (current_value or 0) + op.value
        if op.op == PatchOpType.DECREMENT:
            return (current_value or 0) - op.value
        if op.op == PatchOpType.TOGGLE:
            return not current_value if isinstance(current_value, bool) else bool(op.value)
        raise ValueError(f"Unsupported op: {op.op}")

    def list_paths(self, prefix: str = "") -> List[str]:
        return [p for p in self._data if p.startswith(prefix)]

    def create_snapshot(self) -> None:
        pass

    def release_snapshot(self) -> None:
        pass

    def close(self) -> None:
        self._data.clear()
