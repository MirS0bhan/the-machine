# Runtime Model — Agent-Driven UI via MCP

This document describes the runtime architecture implemented by The Machine.

## Boot Path

The ISO initramfs launches services in layer order (L0 → L3 → L2 → L1 → L4 → L5):

1. **system-daemon** — evdev input with provenance markers; kernel-adjacent I/O MCP surface
2. **mcp-bus** — message fabric, intent registry, policy middleware
3. **policy-broker** — capability enforcement, audit, confirmation UI bridge
4. **state-store**, **event-bus**, **lambda-server**, **local-model-daemon**, **marketplace** — L1/L4 primitives
5. **agent-core** — LLM-backed planner (`localmodel.*` + optional cloud via `OPENAI_API_KEY`)
6. **compositor** + **ui-runtime** — framebuffer present loop + declarative UI tree

Init keeps PID 1 alive while services run; **fallback-shell** provides emergency console access.

Environment defaults are set in `build/mkinitramfs.sh` (`STATE_STORE_BACKEND=sled`, `LOCAL_MODEL_PATH=/models/machine-tiny.gguf`, `WAYLAND_DISPLAY=wayland-0`).

## User Request Flow

```
User input → Event Bus → Agent Core wake
                ↓
         bus.resolve(method)
           /          \
      hit              miss
       ↓                ↓
  forward to       lambda.register (sandboxed synthesis)
  registered            +
  handler          ui.patch (materialize widgets)
       ↓                ↓
  UI binding      bindings → MCP on widget events
  (mcp:calc.add)
```

1. **Resolve** — Agent (or UI engine directly for `mcp:` bindings) calls `bus.resolve(method)` against the MCP intent registry.
2. **Hit** — Bus forwards to the registered handler (lambda-server, state-store, etc.).
3. **Miss** — Agent synthesizes a sandboxed lambda, registers it with `exposes_mcp`, which hot-registers routes via `_bus.register` (internal, policy-gated).
4. **Materialize** — Agent calls `ui.patch` to insert widgets whose `bindings` reference the new MCP methods.
5. **Steady state** — Subsequent UI events invoke bindings directly through the bus without re-invoking the agent.

## Proactive Scheduler

The Event Bus wakes the agent independently of user input:

| Source | Mechanism | Example |
|--------|-----------|---------|
| Timers / cron | `event.schedule` | `@every 30s` heartbeat |
| Heartbeat | Built-in loop in event-bus | `scheduler.heartbeat.tick` with environment snapshot |
| D-Bus signals | `event-bus` dbus adapter (native zbus) | `desktop.notify`, `login.prepare_sleep` |
| Filesystem events | `event-bus` inotify adapter | `fs.change.<pattern>` |
| Audio | `event-bus` audio adapter | `pipewire.state` |

Heartbeat payloads include an **environment snapshot** (uptime, hostname, lambda/UI/policy health). The agent persists this to `system.environment` in the State Store and may act on it during `process_wake`.

## Policy & Confirmation

All non-exempt MCP calls pass through **policy.check** middleware on the bus. Registrations use **policy.validate_register** before `_bus.register` inserts a route.

Sensitive operations surface a **confirmation UI** on a compositor-protected layer (`compositor.confirmation.set_active`). User approval flows `ui.event` → `policy.confirm`.

## MCP Registry

Per `mcp-bus-spec.md`:

- **Namespaces:** `mcp-intent`, `event-handler`, `system-op`, `state-op`
- **Pattern matching:** `calc.*` matches `calc.add`, `calc.eval`, etc.
- **Registration:** Side effect of `lambda.register` (via `_bus.register`); broker-validated
- **Persistence:** Boot reload from `perm.mcp_routes.*` in state-store
- **Introspection:** `bus.resolve`, `bus.list_routes`

## Python ↔ Rust

Rust binaries ship in the initramfs for boot. Python servers remain the reference implementation for tests and agent development. See [python-rust-overlap.md](../guides/python-rust-overlap.md).

## Remaining Work

See [Gap Analysis](./gap-analysis.md) and [Expansion Proposal](./expansion-proposal.md). Platform polish items (G12–G14, G17 wlroots session) remain; critical path gaps G1/G6/G16 are closed.
