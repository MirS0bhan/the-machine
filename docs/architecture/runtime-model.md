# Runtime Model — Agent-Driven UI via MCP

This document describes the target runtime architecture implemented (incrementally) by The Machine.

## Boot Path

The ISO initramfs launches services in layer order (L0 → L3 → L2 → L1 → L4 → L5):

1. **system-daemon** — kernel-adjacent I/O and power/display/net MCP surface
2. **mcp-bus** — message fabric and intent registry
3. **policy-broker** — capability enforcement
4. **state-store**, **event-bus**, **lambda-server** — L1 primitives
5. **agent-core** — decision harness (heuristic today; LLM-backed in full deployment)
6. **compositor** + **ui-runtime** — L5 display stack (compositor is model-layer today; wlroots integration is planned)

Init keeps PID 1 alive while services run; **fallback-shell** provides emergency console access.

## User Request Flow

```
User input → Event Bus → Agent Core wake
                ↓
         bus.resolve(method)
           /          \
      hit              miss
       ↓                ↓
  forward to       lambda.register (sandboxed)
  registered            +
  handler          ui.patch (materialize widgets)
       ↓                ↓
  UI binding      bindings → MCP on widget events
  (mcp:calc.add)
```

1. **Resolve** — Agent (or UI engine directly for `mcp:` bindings) calls `bus.resolve(method)` against the MCP intent registry.
2. **Hit** — Bus forwards to the registered handler (lambda-server, state-store, etc.).
3. **Miss** — Agent synthesizes a sandboxed lambda, registers it with `exposes_mcp`, which hot-registers routes via `_bus.register` (internal, not agent-callable).
4. **Materialize** — Agent calls `ui.patch` to insert widgets whose `bindings` reference the new MCP methods.
5. **Steady state** — Subsequent UI events invoke bindings directly through the bus without re-invoking the agent.

## Proactive Scheduler

The Event Bus wakes the agent independently of user input:

| Source | Mechanism | Example |
|--------|-----------|---------|
| Timers / cron | `event.schedule` | `@every 30s` heartbeat |
| Heartbeat | Built-in loop in event-bus | `scheduler.heartbeat.tick` with environment snapshot |
| D-Bus signals | Planned adapter | `org.freedesktop` notifications |
| Filesystem events | Planned adapter | `inotify` on watched paths |

Heartbeat payloads include an **environment snapshot** (uptime, hostname, timestamp). The agent persists this to `system.environment` in the State Store and may act on it during `process_wake`.

## MCP Registry

Per `mcp-bus-spec.md`:

- **Namespaces:** `mcp-intent`, `event-handler`, `system-op`, `state-op`
- **Pattern matching:** `calc.*` matches `calc.add`, `calc.eval`, etc.
- **Registration:** Side effect of `lambda.register` (via `_bus.register`); not exposed to agents
- **Introspection:** `bus.resolve`, `bus.list_routes`

## Python ↔ Rust

Rust binaries ship in the initramfs for boot. Python servers remain the reference implementation for tests and agent development. See [python-rust-overlap.md](../guides/python-rust-overlap.md).

## Remaining Work

- **wlroots compositor** — real Wayland session instead of model-only compositor
- **D-Bus / fs event adapters** — discrete event registrations in event-bus
- **LLM integration** — wire `local-model` into agent-core `classify` / `plan`
- **Policy gate on `_bus.register`** — broker validation before route insertion
