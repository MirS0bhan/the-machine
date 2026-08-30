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
- [x] **G1** Production cloud API key management (`agent-core/secrets.rs`, file + env, policy-gated)
- [x] **G6** Rust AUIL parser in boot path (`ui-runtime/auil.rs`, `build/boot.auil`, `ui.auil.load`)
- [x] **G16** DRM/KMS compositor backend (`compositor/drm.rs`, auto-select via `THE_MACHINE_COMPOSITOR_BACKEND`)
- [x] **D3** MCP wire protocol docs aligned to NDJSON (`docs/components/mcp-bus.md` documents newline-delimited JSON)
- [x] **D4** README/runtime-model sync (`make verify-docs` + component-inventory.yaml)
- [x] **C1** CI release bundle rust-coverage artifact trap (`build/assemble-release.sh`)
- [x] **C2** Dev-harness sockets ignore `THE_MACHINE_SOCKET_DIR` (`common::paths`, bus forward)
- [x] **D5** Component See Also links resolve to sibling pages (`verify-docs-code.py`)
- [x] **G7** Native zbus D-Bus adapter (`event-bus/src/adapters/dbus.rs`; replaces `dbus-monitor`)

---

## Open — Critical Path

_None — all P0/P1 gaps from the expansion campaign are closed._

---

## Open — Platform

| ID | Gap | Component | Priority |
|----|-----|-----------|----------|
| G12 | `bus.lease` metadata only (no socket fast-path yet) | mcp-bus | P3 |
| G13 | Rootfs installer needs debootstrap + kernel on target HW | build | P3 |
| G14 | system-daemon mutations (wifi, display mode, netlink) still refuse; reads use sysfs/proc | system-daemon | P3 |
| G17 | Full wlroots Wayland session (DRM dumb buffer works today) | compositor | P3 |

---

## Open — Documentation

_None._

---

## How to Use

1. Pick a gap ID from the platform table.
2. Implement + test + update this checklist.
3. Reference the gap ID in PR description.
4. See [Expansion Proposal](./expansion-proposal.md) for phased roadmap.
