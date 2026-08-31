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
- [x] **G14 (partial)** `power.set_profile` via cpufreq sysfs (`system-daemon/src/power.rs`)
- [x] **G14 (partial)** Display modes via sysfs/DRM; `display.set_mode` on DRM hosts; `net.list_interfaces` via sysfs; `ip link` for up/down
- [x] **G14 (partial)** `audio.set_default` via `pactl set-default-sink` (`system-daemon/src/audio.rs`)
- [x] **G14 (partial)** `net.set_interface_state` via rtnetlink RTM_SETLINK (`system-daemon/src/netlink.rs`)
- [x] **G14 (partial)** udev hotplug → `event.publish` via kernel uevent netlink (`system-daemon/src/hotplug.rs`)
- [x] **G13** Rootfs installer: debootstrap packages, kernel in `/boot`, GRUB `LABEL=the-machine` + fstab (`build/mkrootfs.sh`, `build/installer/install.sh`); loopback GRUB validation (`build/test-installer-grub.sh`); `boot.auil` at `/etc/the-machine/boot.auil`; operator installed-rootfs validation (`build/validate-installed-rootfs.sh`) (#157, #159, #168)

---

## Open — Critical Path

_None — all P0/P1 gaps from the expansion campaign are closed._

---

## Open — Platform

| ID | Gap | Component | Priority |
|----|-----|-----------|----------|
| G17 | xdg-shell via wayland-protocols `xdg_wm_base` v5 (implementation in #215; **mark closed only after merge**). wlroots/XWayland remain non-goals. | compositor | P3 |

---

## Open — Documentation

_None._

---

## How to Use

1. Pick a gap ID from the platform table.
2. Implement + test + update this checklist.
3. Reference the gap ID in PR description.
4. See [Expansion Proposal](./expansion-proposal.md) for phased roadmap.
