# Python ↔ Rust Overlap Guide

The Machine was started in **Python** (MCP servers, FastAPI, pytest) and is being
rewritten in **Rust** (Unix-socket daemons for the boot/ISO path). Several components
exist in **both languages at the same time**. This is intentional during migration —
not a bug — but you must know which copy is authoritative for each workflow.

## Quick rule

| Workflow | Use |
|----------|-----|
| `make test-python`, integration tests, agent prototyping | **Python** where marked below |
| `make iso`, `make qemu`, `scripts/start-services.sh` (default) | **Rust** daemons |
| Reading the spec / designing a feature | `docs/*-spec.md` (language-agnostic) |

Set `THE_MACHINE_RUNTIME=python|rust|hybrid` when starting services (see below).

---

## Component matrix

| Component | Python path | Rust path | Canonical — tests | Canonical — boot/ISO | Migration |
|-----------|-------------|-----------|-------------------|----------------------|-----------|
| **Lambda Server** | `lambda-server/*.py` | `lambda-server/src/` | **Python** | **Rust** (seccomp sandbox) | Rust absorbing sandbox; Python keeps agent dev ergonomics |
| **Policy Broker** | `policy-broker/policy_broker/` | `policy-broker/src/` | **Python** | Python preferred* | Port rule engine to Rust; delete Python when parity reached |
| **State Store** | `state-store/state_store/` | `state-store/src/` | **Python** | Rust (in-mem stub) | Port RocksDB + watch to Rust |
| **Event Bus** | `event-bus/event_bus/` | `event-bus/src/` | Python (in-proc router) | **Rust** (full scheduler) | Python harness shrinks to test doubles only |
| **UI rendering** | `ui-engine/` (AUIL/ASL) | `ui-runtime/` (patch tree) | **Python** | **Rust** (daemon) | Share patch grammar; ui-engine stays reference parser |
| **Agent Core** | — (removed) | `agent-core/` | **Rust** | **Rust** | Python `agent/` never landed; Rust only |
| **Local Model** | `local-model/` | — | **Python** | stub in Rust agent | Future Rust or sidecar |
| **MCP Bus** | — | `mcp-bus/` | **Rust** | **Rust** | Python never existed |
| **System Daemon** | — | `system-daemon/` | **Rust** | **Rust** | Python never existed |
| **Compositor** | — | `compositor/` | **Rust** | **Rust** | Python never existed |
| **Fallback Shell** | — | `fallback-shell/` | **Rust** | **Rust** | Python never existed |

\* Boot initramfs currently ships **Rust** policy-broker (deny-by-default stub). For a
working policy-gated dev stack, use `THE_MACHINE_RUNTIME=hybrid` or `python`.

---

## What each duplicate actually does today

### Lambda Server — biggest overlap

| | Python | Rust |
|---|--------|------|
| Registry, search, MCP tools | ✅ full | ✅ partial |
| In-process `pure` execution | ✅ `executor.py` | via sandbox |
| Process supervisor / warm pool | ✅ `supervisor.py` (simulated) | ✅ `sandbox.rs` (real namespaces/seccomp) |
| HTTP API | ✅ `http_server.py` | ❌ |
| Tests | `test_server.py`, integration | `cargo test` (minimal) |

**Do not** assume registering a function in Python makes it visible to the Rust daemon —
they do not share storage.

### Policy Broker

| | Python | Rust |
|---|--------|------|
| Rule interpreter | ✅ `interpreter.py` | ❌ deny-by-default stub |
| Audit log | ✅ | partial |
| Rate limiting | ✅ | ❌ |
| Unix socket MCP | ❌ (FastAPI HTTP) | ✅ |

Integration tests use the **Python interpreter directly** (in-process), not either daemon.

### State Store

| | Python | Rust |
|---|--------|------|
| Persistence | RocksDB or `MemoryBackend` | in-memory `HashMap` only |
| `state.watch` | SSE polling | broadcast stub |
| MCP transport | FastAPI HTTP | Unix socket |

### Event Bus

| | Python | Rust |
|---|--------|------|
| `EventRouter.publish()` | ✅ test harness | ✅ full daemon |
| Cron / scheduler | ❌ | ✅ |
| Agent-wake coalescing | ❌ | ✅ |
| Integration tests | ✅ in-proc `EventRouter` | ❌ not wired |

The Python package exists so `tests/integration/` can import `event_bus.router` without
spawning the Rust binary. It is **not** a second production implementation.

### UI: `ui-engine` vs `ui-runtime`

- **`ui-engine` (Python)** — canonical AUIL parser, patch protocol, renderer, demo app.
- **`ui-runtime` (Rust)** — holds an in-memory tree on the boot path; should consume the
  same patch grammar but does not embed the Python parser.

---

## Runtime selection

`scripts/start-services.sh` respects `THE_MACHINE_RUNTIME`:

```bash
THE_MACHINE_RUNTIME=rust    # default — ISO/boot path, all Rust daemons
THE_MACHINE_RUNTIME=python  # Python MCP servers on HTTP ports (dev only)
THE_MACHINE_RUNTIME=hybrid  # Rust bus + daemons, Python policy-broker + lambda-server
```

| Variable | Effect |
|----------|--------|
| `THE_MACHINE_SOCKET_DIR` | Unix socket directory (default `/tmp/the-machine/run`) |
| `STATE_STORE_BACKEND=memory` | Force Python state-store off RocksDB |
| `LOCAL_MODEL_PATH` | GGUF path; omit for stub mode |

---

## Directory layout (dual components)

```
lambda-server/
├── *.py              ← Python reference (tests, HTTP, agent dev)
├── src/              ← Rust daemon (ISO, sandbox)
└── README.md         ← start here

policy-broker/
├── policy_broker/    ← Python reference (tests, rule engine)
├── src/              ← Rust stub (boot placeholder)
└── README.md

state-store/
├── state_store/      ← Python reference (persistence)
├── src/              ← Rust in-memory daemon
└── README.md

event-bus/
├── event_bus/        ← Python test harness ONLY
├── src/              ← Rust production daemon
└── docs/spec.md
```

---

## Contributing during migration

1. **New feature?** Implement in the language that is canonical for your target:
   - Boot-critical / security → Rust
   - Agent tooling / rapid iteration → Python
2. **Fixing a test failure?** Check which implementation the test imports (see
   `tests/integration/conftest.py` — mostly Python in-process).
3. **Do not delete Python** until the Rust port passes the same test suite via Unix sockets.
4. **Do not duplicate business logic** — port behavior, then delete the old copy.
5. Regenerate docs after changes: `make docs`

---

## Known stale references

These still describe a Python-only world and are being updated:

- `docs/book/index.md` §1 "Running a component" — see this guide instead
- `docs/build/book.md` — generated; run `make docs` after edits
- Historical mention of `agent/` Python package — replaced by `agent-core/` Rust crate

See also: [Development Guide](./development.md) · [Getting Started](./getting-started.md)
