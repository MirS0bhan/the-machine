"""In-memory state backend used when RocksDB is unavailable."""

from __future__ import annotations

import json
from typing import Any, Dict, List, Optional

from .models import PatchOp, StateResponse


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

  def patch(self, ops: List[PatchOp]) -> List[Dict[str, Any]]:
      results: List[Dict[str, Any]] = []
      for op in ops:
          if op.op.value == "=":
              rev = self.put(op.path, op.value)
              results.append({"path": op.path, "revision": rev})
      return results

  def list_paths(self, prefix: str = "") -> List[str]:
      return [p for p in self._data if p.startswith(prefix)]

  def close(self) -> None:
      self._data.clear()
