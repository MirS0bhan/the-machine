"""
Shared fixtures for cross-component integration tests.

Provides in-process instances of every L1/L2 component wired together,
plus mock helpers for policy-gating workflows.

Components under test:
    - Lambda Server  (MCPControlInterface)
    - State Store    (in-memory dict backend, mimics RocksDB)
    - Event Bus      (EventRouter)
    - Policy Broker  (PolicyInterpreter + AuditLogger)
"""

import asyncio
import sys
import types
from pathlib import Path
from typing import Any, Dict, List, Optional
from datetime import datetime

import pytest

# ---------------------------------------------------------------------------
# Ensure component packages are importable (they live in subdirectories,
# not on the normal PYTHONPATH when running from the repo root).
# ---------------------------------------------------------------------------
_LAMBDA_DIR = Path(__file__).resolve().parents[2] / "lambda-server"
_POLICY_DIR = Path(__file__).resolve().parents[2] / "policy-broker"
_STATE_DIR = Path(__file__).resolve().parents[2] / "state-store"
_EVENT_DIR = Path(__file__).resolve().parents[2] / "event-bus"

for _d in (_LAMBDA_DIR, _POLICY_DIR, _STATE_DIR, _EVENT_DIR):
    _p = str(_d)
    if _p not in sys.path:
        sys.path.insert(0, _p)

# Fix imports so modules resolve relative to their own package dirs
# (e.g. `from models import ...` inside lambda-server/).
import importlib  # noqa: E402

for _mod_name, _pkg_dir in [
    ("models", str(_LAMBDA_DIR)),
    ("registry", str(_LAMBDA_DIR)),
    ("enforcer", str(_LAMBDA_DIR)),
    ("executor", str(_LAMBDA_DIR)),
    ("supervisor", str(_LAMBDA_DIR)),
    ("router", str(_LAMBDA_DIR)),
    ("mcp_interface", str(_LAMBDA_DIR)),
    ("config", str(_LAMBDA_DIR)),
]:
    _fqn = f"lambda_server.{_mod_name}"
    if _fqn not in sys.modules:
        _spec = importlib.util.spec_from_file_location(_fqn, f"{_pkg_dir}/{_mod_name}.py")
        _mod = importlib.util.module_from_spec(_spec)
        sys.modules[_fqn] = _mod
        _spec.loader.exec_module(_mod)

# Now build a proper `lambda_server` package so `from lambda_server.xxx` works
if "lambda_server" not in sys.modules:
    _pkg = types.ModuleType("lambda_server")
    _pkg.__path__ = [str(_LAMBDA_DIR)]
    _pkg.__package__ = "lambda_server"
    sys.modules["lambda_server"] = _pkg
    # Re-export submodules
    for _sub in ("models", "registry", "enforcer", "executor", "supervisor",
                 "router", "mcp_interface", "config"):
        _fqn = f"lambda_server.{_sub}"
        if _fqn in sys.modules:
            setattr(_pkg, _sub, sys.modules[_fqn])

# Policy-broker imports (already a proper package)
from policy_broker.models import (  # noqa: E402
    PolicyDoc,
    Rule,
    CheckRequest,
    CheckResponse,
    AuditEntry,
)
from policy_broker.interpreter import PolicyInterpreter  # noqa: E402
from policy_broker.audit import AuditLogger  # noqa: E402
from policy_broker.state_store import StateStoreClient  # noqa: E402

from event_bus.router import EventRouter  # noqa: E402
from event_bus.models import EventPublishRequest  # noqa: E402


# ===================================================================
# In-memory State Store (replaces RocksDB for testing)
# ===================================================================

class InMemoryStateStore:
    """Minimal in-memory state store used by tests and Policy Broker."""

    def __init__(self) -> None:
        self._data: Dict[str, Any] = {}
        self._revisions: Dict[str, int] = {}
        self._audit_log: List[AuditEntry] = []
        self._next_revision: int = 1
        self._watchers: Dict[str, List[asyncio.Queue]] = {}

    # -- state operations --------------------------------------------------
    def get(self, path: str) -> Optional[Dict[str, Any]]:
        if path not in self._data:
            return None
        return {"value": self._data[path], "revision": self._revisions[path]}

    def put(self, path: str, value: Any) -> Dict[str, Any]:
        self._data[path] = value
        rev = self._next_revision
        self._next_revision += 1
        self._revisions[path] = rev
        # Notify watchers
        for q in self._watchers.get(path, []):
            q.put_nowait({"value": value, "revision": rev, "path": path})
        return {"value": value, "revision": rev}

    def patch(self, ops: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        results: List[Dict[str, Any]] = []
        for op in ops:
            path = op["path"]
            value = op.get("value")
            result = self.put(path, value)
            results.append(result)
        return results

    # -- audit log ---------------------------------------------------------
    def append_audit_log(self, entry: AuditEntry) -> None:
        self._audit_log.append(entry)

    def query_audit_log(self, filter: Optional[Dict] = None) -> List[AuditEntry]:
        if not filter:
            return list(self._audit_log)
        results = []
        for e in self._audit_log:
            match = True
            for k, v in filter.items():
                if getattr(e, k, None) != v:
                    match = False
                    break
            if match:
                results.append(e)
        return results

    # -- policy doc storage (used by PolicyBroker's StateStoreClient) ------
    def get_policy(self, key: str) -> Optional[PolicyDoc]:
        return self._data.get(f"__policy__{key}")

    def put_policy(self, key: str, doc: PolicyDoc) -> None:
        self._data[f"__policy__{key}"] = doc

    # -- watcher API -------------------------------------------------------
    def watch(self, path: str) -> asyncio.Queue:
        q: asyncio.Queue = asyncio.Queue()
        self._watchers.setdefault(path, []).append(q)
        return q


# ===================================================================
# Policy Broker wired to the in-memory state store
# ===================================================================

class InMemoryStateStoreClient(StateStoreClient):
    """StateStoreClient that delegates to InMemoryStateStore."""

    def __init__(self, store: InMemoryStateStore) -> None:
        super().__init__()
        self._store = store

    def get_policy(self, policy_key: str) -> Optional[PolicyDoc]:
        return self._store.get_policy(policy_key)

    def put_policy(self, policy_key: str, policy_doc: PolicyDoc) -> None:
        self._store.put_policy(policy_key, policy_doc)

    def append_audit_log(self, entry: AuditEntry) -> None:
        self._store.append_audit_log(entry)

    def query_audit_log(self, filter: Optional[Dict] = None) -> List[AuditEntry]:
        return self._store.query_audit_log(filter)


# ===================================================================
# Fixtures
# ===================================================================

@pytest.fixture
def state_store() -> InMemoryStateStore:
    """In-memory state store instance."""
    return InMemoryStateStore()


@pytest.fixture
def event_bus() -> EventRouter:
    """In-process event bus router."""
    return EventRouter()


@pytest.fixture
def policy_store_client(state_store: InMemoryStateStore) -> InMemoryStateStoreClient:
    """StateStoreClient wired to the in-memory state store."""
    return InMemoryStateStoreClient(state_store)


@pytest.fixture
def policy_broker(policy_store_client: InMemoryStateStoreClient) -> PolicyInterpreter:
    """Policy Interpreter wired to the in-memory state store."""
    return PolicyInterpreter(policy_store_client)


@pytest.fixture
def lambda_mcp():
    """Lambda Server MCP control interface (in-process)."""
    from mcp_interface import MCPControlInterface
    return MCPControlInterface()


# ===================================================================
# Convenience: register a policy and return the broker
# ===================================================================

@pytest.fixture
def register_policy(policy_broker):
    """Helper that registers a PolicyDoc and returns the broker."""

    def _register(
        rules: List[Dict[str, Any]],
        policy_key: str = "default",
    ) -> PolicyInterpreter:
        doc = PolicyDoc(
            rules=[
                Rule(
                    path=r.get("path", "*"),
                    method=r["method"],
                    decision=r.get("decision", "ALLOW"),
                    capabilities=r.get("capabilities", []),
                    rate_limit=r.get("rate_limit"),
                )
                for r in rules
            ]
        )
        policy_broker.register(doc, policy_key)
        return policy_broker

    return _register
