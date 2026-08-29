# Development Guide

## Repository layout

The Machine uses a **hybrid architecture**:

- **Rust daemons** — production boot path (Unix sockets at `/run/the-machine/*.sock`)
- **Python MCP servers** — reference implementations with full test coverage

| Component | Rust crate | Python package | Canonical for tests |
|-----------|------------|----------------|---------------------|
| MCP Bus | `mcp-bus` | — | Rust |
| System Daemon | `system-daemon` | — | Rust |
| Policy Broker | `policy-broker` (stub) | `policy-broker` | **Python** |
| State Store | `state-store` (in-mem) | `state-store` | Python (memory fallback) |
| Event Bus | `event-bus` | `event-bus` | Python (integration) |
| Lambda Server | `lambda-server` | `lambda-server` | **Python** |
| Agent Core | `agent-core` | — | Rust |
| Local Model | — | `local-model` | Python |
| UI Engine | — | `ui-engine` | Python |
| UI Runtime | `ui-runtime` | — | Rust |
| Compositor | `compositor` | — | Rust |
| Fallback Shell | `fallback-shell` | — | Rust |

## MCP socket layout

All components communicate via JSON-line MCP over Unix domain sockets:

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

The MCP Bus forwards requests to the appropriate component socket based on its method registry.

## Adding a component

1. Write the design spec in `docs/<component>-spec.md`
2. Implement the Rust daemon (if boot-critical) or Python MCP server
3. Register methods in `mcp-bus/src/main.rs`
4. Add integration tests in `tests/integration/`
5. Regenerate docs: `make docs`

## CI checklist

```bash
make build
make test-all
make docs
make initramfs
make iso
```
