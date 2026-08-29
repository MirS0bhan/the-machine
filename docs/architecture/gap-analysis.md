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

---

## Open — Critical Path

| ID | Gap | Component | Priority |
|----|-----|-----------|----------|
| G1 | Agent uses heuristic classifier, not LLM | agent-core + local-model | P0 |
| G2 | No wlroots / Wayland pixels | compositor | P0 |
| G3 | Rust policy-broker is deny-by-default stub | policy-broker | P0 |
| G4 | MCP registry persist on register; rebuild on bus restart not yet implemented | mcp-bus + state-store | P1 |
| G5 | UI input not routed compositor → ui.event | compositor + ui-runtime | P1 |
| G6 | Python AUIL parser not in boot path | ui-engine → ui-runtime | P1 |

---

## Open — Platform

| ID | Gap | Component | Priority |
|----|-----|-----------|----------|
| G7 | No D-Bus event adapter | event-bus | P2 |
| G8 | No filesystem watch adapter | event-bus | P2 |
| G9 | No OCI/Firecracker sandbox | lambda-server | P2 |
| G10 | Rust state-store in-memory only | state-store | P2 |
| G11 | `local-model` not in initramfs | build + local-model | P2 |
| G12 | No fast-path MCP leases | mcp-bus | P3 |
| G13 | No rootfs / installer (initramfs only) | build | P3 |
| G14 | system-daemon input is stub | system-daemon | P3 |
| G15 | Broker confirmation UI not implemented | policy-broker | P3 |

---

## Open — Documentation

| ID | Gap | Location |
|----|-----|----------|
| D1 | ~~`mcp-bus-spec.md` status still "pre-implementation"~~ | docs/mcp-bus-spec.md |
| D2 | ~~`components/mcp-bus.md` references `bus.registry.lookup`~~ | docs/components/mcp-bus.md |
| D3 | Wire protocol section describes length-prefix; impl uses newline JSON | docs/components/mcp-bus.md |

---

## How to Use

1. Pick a gap ID from the critical path.
2. Implement + test + update this checklist.
3. Reference the gap ID in PR description.
4. See [Expansion Proposal](./expansion-proposal.md) for phased roadmap.
