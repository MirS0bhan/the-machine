# Development Guide

## Python ↔ Rust overlap (read this first)

The repo is mid-migration from Python MCP servers to Rust boot daemons. Several
components exist in both languages. **Do not assume they share state or behavior.**

→ **[Python ↔ Rust Overlap Guide](./python-rust-overlap.md)** — canonical matrix, migration status, pitfalls.

## Repository layout

The Machine uses a **hybrid architecture** during migration:

- **Rust daemons** — boot path, ISO, Unix sockets at `/run/the-machine/*.sock`
- **Python packages** — reference implementations, integration tests, agent prototyping

| Component | Rust crate | Python package | Tests | Boot/ISO | Status |
|-----------|------------|----------------|-------|----------|--------|
| MCP Bus | `mcp-bus` | — | Rust | Rust | Rust only |
| System Daemon | `system-daemon` | — | Rust | Rust | Rust only |
| Policy Broker | `policy-broker` (stub) | `policy-broker` | **Python** | Rust† | Porting |
| State Store | `state-store` (in-mem) | `state-store` | **Python** | Rust† | Porting |
| Event Bus | `event-bus` | `event_bus` (harness) | Python‡ | **Rust** | Rust leads |
| Lambda Server | `lambda-server` | `lambda-server` | **Python** | **Rust** | Dual |
| Agent Core | `agent-core` | — (removed) | Rust | Rust | Rust only |
| Local Model | — | `local-model` | Python | Python | Python only |
| UI Engine | — | `ui-engine` | **Python** | — | Python reference |
| UI Runtime | `ui-runtime` | — | Rust | Rust | Rust daemon |
| Compositor | `compositor` | — | Rust | Rust | Rust only |
| Fallback Shell | `fallback-shell` | — | Rust | Rust | Rust only |

† Boot ships Rust stub; use `THE_MACHINE_RUNTIME=hybrid` for full policy-gated dev.  
‡ Integration tests use Python `EventRouter` in-process, not the Rust daemon.

## MCP socket layout

All Rust components communicate via JSON-line MCP over Unix domain sockets:

```
/run/the-machine/mcp-bus.sock      # L3 message fabric (entry point)
/run/the-machine/policy-broker.sock
/run/the-machine/state-store.sock
/run/the-machine/event-bus.sock
/run/the-machine/lambda-server.sock
/run/the-machine/agent-core.sock
/run/the-machine/ui-runtime.sock
/run/the-machine/compositor.sock
/run/the-machine/system-daemon.sock
/run/the-machine/fallback-shell.sock
```

Python servers use HTTP (FastAPI) on separate ports during dev — they are **not** on this socket bus unless bridged.

### Policy fail-closed and grant tokens

- The bus **fails closed** for mutations and `_bus.register` when `policy-broker.sock` is down. Read-only boot methods (`state.get`, `*.status`, `*.health`, …) still proceed. Set `THE_MACHINE_POLICY_FAIL_OPEN=1` only in throwaway local experiments.
- `scripts/start-services.sh` starts the policy broker **before** the bus so the first forwarded calls have a live decision point.
- System-daemon mutations require a broker-issued HMAC grant token (`params.token`). Share the key via `THE_MACHINE_TOKEN_SECRET` or `/run/the-machine/secrets/token` (mode 0600). The ISO default material is `the-machine-grant-token-v1` until a unique secret is written.
- External MCP: `bus.external.register` requires `https://` (or `http://localhost`) and an explicit `allowed_methods` list — no `*`.

## Runtime selection

```bash
THE_MACHINE_RUNTIME=rust    # default — matches ISO boot
THE_MACHINE_RUNTIME=hybrid  # Rust daemons + Python policy-broker & lambda-server
THE_MACHINE_RUNTIME=python  # Python HTTP servers only (no socket bus)
./scripts/start-services.sh
```

## Adding a component

1. Write the design spec in `docs/<component>-spec.md`
2. Pick language: boot-critical → Rust; agent tooling → Python (port later)
3. If dual, add a `README.md` with the overlap table
4. Register Rust MCP methods in `mcp-bus/src/main.rs`
5. Add integration tests in `tests/integration/`
6. Regenerate docs: `make docs`

## CI checklist

```bash
make verify        # recommended: full local verification
make lint          # rustfmt + clippy (mcp-bus)
make build
make test-all
make docs
make initramfs-release
make iso-release
```
