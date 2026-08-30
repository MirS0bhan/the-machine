# Gap Analysis — Living Checklist

Tracks known gaps between the **north-star** (fully agentic Linux OS) and the current codebase. Updated alongside implementation PRs.

**Last reviewed:** 2026-08-30

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
- [x] **G7** Native D-Bus adapter via `zbus` (`event-bus/src/adapters/dbus.rs`; no `dbus-monitor` dependency)
- [x] Policy middleware fail-closed for mutations when broker is down (`THE_MACHINE_POLICY_FAIL_OPEN=1` override)
- [x] System-daemon mutations verify HMAC grant tokens (`common::token`)
- [x] `bus.external.register` rejects open-proxy URLs/wildcards
- [x] Synthesized Python lambdas use a shebang script path (sandbox bind-mount)
- [x] Lambda entrypoints must live under `THE_MACHINE_LAMBDA_DIR`
- [x] Marketplace bundle HMAC check + no `eval(` in pack sources
- [x] `shell.*` / `hello` routed to fallback-shell
- [x] `net.get_wifi_status` implemented; `connect_wifi` no longer returns `status: null`
- [x] **G12** `bus.lease` optional fast-path relay socket when `THE_MACHINE_LEASE_FAST_PATH=1` (`mcp-bus/src/lease.rs`)
- [x] **G13 (partial)** Rootfs installer + `build/rootfs-validate.sh` CI validation (`build/mkrootfs.sh`, `build/installer/install.sh`)
- [x] **G14 (partial)** rtnetlink list/set link; wifi via `wpa_cli`; PipeWire via `pactl`; display + power done
- [x] **G17 (partial)** Wayland globals + `wl_shm` surface commit → pixel paint (`compositor/src/wl_globals.rs`, `wl_shm.rs`)

---

## Open — Critical Path

_None — all P0/P1 gaps from the expansion campaign are closed._

---

## Open — Platform

| ID | Gap | Component | Priority |
|----|-----|-----------|----------|
| G13 | Rootfs validated in CI; full debootstrap boot needs target-HW smoke test | build | P3 |
| G17 | Full wlroots compositing (xdg-shell, input routing); SHM paint path done | compositor | P3 |

_None critical — G14 rtnetlink + wifi/audio closed._

---

## Open — Documentation

_None._

---

## How to Use

1. Pick a gap ID from the platform table.
2. Implement + test + update this checklist.
3. Reference the gap ID in PR description.
4. See [Expansion Proposal](./expansion-proposal.md) for phased roadmap.
