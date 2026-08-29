# Gap Analysis — Living Checklist

Tracks known gaps between the **north-star** (fully agentic Linux OS) and the current codebase. Updated alongside implementation PRs.

**Last reviewed:** 2026-08-29

---

## Closed (recent)

- [x] Dynamic MCP registry with wildcard patterns (`calc.*`)
- [x] `_bus.register` side effect from `lambda.register` (Rust)
- [x] `bus.resolve` / `bus.list_routes` introspection
- [x] Agent resolve → miss → synthesize → `ui.patch` loop
- [x] UI widget `mcp:` / `state:` binding execution (`ui.event`)
- [x] Scheduler heartbeat with environment snapshot
- [x] Initramfs keeps compositor + ui-runtime running
- [x] Python lambda-server bus registration (`bus_client.py`)
- [x] `handles_event` manifest → bus + event-bus registration
- [x] Rust policy-broker rule engine ported (interpreter parity, audit, confirmation)
- [x] MCP broker middleware gates all non-exempt MCP calls via `policy.check`
- [x] Registry lifecycle: `_bus.deregister`, `lambda.deprecate`, boot reload from `perm.mcp_routes.*`
- [x] Rust state-store sled persistence + `state.watch` broadcast subscriptions
- [x] **Phase 1:** `local-model-daemon` on MCP bus; agent classify/plan via `localmodel.*`; cloud router; skills from state
- [x] **Phase 2:** Compositor input→widget bridge; UI renderer sync; `WAYLAND_DISPLAY` session; state.watch poll
- [x] **Phase 4:** D-Bus/inotify/PipeWire adapters; rich heartbeat aggregation
- [x] **Phase 5:** Warm pools, code synthesis pipeline, semantic `lambda.search` (seccomp sandbox, no OCI)
- [x] **Phase 6:** `mkrootfs.sh`, systemd units, installer script (skeleton)
- [x] **Phase 7:** External MCP proxy, telemetry export, marketplace daemon

---

## Open — Critical Path

| ID | Gap | Component | Priority |
|----|-----|-----------|----------|
| G1 | Agent cloud path needs live API key in production | agent-core | P0 |
| G2 | No wlroots / real Wayland pixels (software surface model only) | compositor | P0 |
| G5 | Real evdev input (simulated pointer loop today) | system-daemon | P1 |
| G6 | Python AUIL parser not embedded in boot path | ui-engine → ui-runtime | P1 |

---

## Open — Platform

| ID | Gap | Component | Priority |
|----|-----|-----------|----------|
| G7 | D-Bus adapter requires `dbus-monitor` on host | event-bus | P2 |
| G11 | GGUF model weights not shipped in initramfs | build + local-model | P2 |
| G12 | `bus.lease` metadata only (no socket fast-path yet) | mcp-bus | P3 |
| G13 | Rootfs installer needs debootstrap + kernel on target HW | build | P3 |
| G14 | system-daemon kernel stubs (display/net/audio) | system-daemon | P3 |
| G15 | Broker confirmation UI not implemented | policy-broker | P3 |

---

## Open — Documentation

| ID | Gap | Location |
|----|-----|----------|
| D3 | Wire protocol section describes length-prefix; impl uses newline JSON | docs/components/mcp-bus.md |

---

## How to Use

1. Pick a gap ID from the critical path.
2. Implement + test + update this checklist.
3. Reference the gap ID in PR description.
4. See [Expansion Proposal](./expansion-proposal.md) for phased roadmap.
