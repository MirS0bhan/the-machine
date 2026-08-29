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
- [x] **Phase 1–7 platform** (LLM, display bridge, events, lambda synthesis, distro skeleton, ecosystem)
- [x] **G2** Framebuffer pixel compositor (`/dev/fb0` mmap + 60fps present loop)
- [x] **G5** Real evdev input with provenance markers (`system-daemon/src/input.rs`)
- [x] **G11** GGUF bundling in initramfs (`build/fetch-model.sh` + `/models/machine-tiny.gguf`)
- [x] **G15** Broker confirmation UI surface (`confirmation_ui.rs` + compositor exclusivity)

---

## Open — Critical Path

| ID | Gap | Component | Priority |
|----|-----|-----------|----------|
| G1 | Agent cloud path needs live API key in production | agent-core | P0 |
| G6 | Python AUIL parser not embedded in boot path | ui-engine → ui-runtime | P1 |

---

## Open — Platform

| ID | Gap | Component | Priority |
|----|-----|-----------|----------|
| G7 | D-Bus adapter requires `dbus-monitor` on host | event-bus | P2 |
| G12 | `bus.lease` metadata only (no socket fast-path yet) | mcp-bus | P3 |
| G13 | Rootfs installer needs debootstrap + kernel on target HW | build | P3 |
| G14 | system-daemon kernel stubs (display/net/audio) | system-daemon | P3 |
| G16 | Full wlroots DRM/KMS compositor (framebuffer works today) | compositor | P3 |

---

## Open — Documentation

| ID | Gap | Location |
|----|-----|----------|
| D3 | ~~Wire protocol section describes length-prefix; impl uses newline JSON~~ | **Closed** — `docs/components/mcp-bus.md` documents newline-delimited JSON |
| D4 | ~~README/runtime-model stale post-Phase-7~~ | **Closed** — `make verify-docs` + component-inventory.yaml |

---

## How to Use

1. Pick a gap ID from the critical path.
2. Implement + test + update this checklist.
3. Reference the gap ID in PR description.
4. See [Expansion Proposal](./expansion-proposal.md) for phased roadmap.
