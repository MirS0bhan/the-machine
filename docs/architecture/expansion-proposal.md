# Expansion Proposal — Toward a Fully Agentic Linux OS

**Version:** 0.1  
**Date:** 2026-08-29  
**Status:** Roadmap proposal  
**Audience:** Contributors, architects, and agent-runtime implementers

---

## Executive Summary

The Machine already implements the **skeleton** of an agent-native OS: MCP as the universal bus, a policy-gated lambda runtime, an event-driven agent wake loop, and a declarative UI engine. What remains is closing the gap between this skeleton and a **fully functional agentic desktop** that boots on real Linux hardware, reasons with LLMs, and materializes arbitrary capabilities through MCP tools rather than fixed applications.

This document proposes a phased expansion plan organized by **impact** and **dependency order**. Each phase is independently shippable; together they complete the vision described in `docs/spec.md` and `docs/architecture/runtime-model.md`.

---

## Current State (Baseline)

| Pillar | Status | Gap |
|--------|--------|-----|
| **Boot / ISO** | Initramfs + 10 Rust daemons + GRUB ISO | No full rootfs; no GPU/display on real hardware |
| **MCP Bus** | Dynamic registry, `bus.resolve`, `_bus.register` | No persistence across restart; no fast-path leases |
| **Agent Core** | Session loop, resolve→synthesize→UI | Heuristic classifier only; no LLM inference |
| **Lambda Server** | Seccomp sandbox (Rust); full registry (Python) | No OCI/Firecracker; limited base images |
| **Policy Broker** | Full Python rule engine | Rust boot stub is deny-by-default |
| **State Store** | Python + RocksDB option | Rust in-memory only in boot path |
| **Event Bus** | Cron, timers, heartbeat, agent wake | No D-Bus, inotify, or PipeWire adapters |
| **UI** | AUIL/ASL parser (Python); patch runtime (Rust) | No pixel compositor; no input→binding hot path |
| **Compositor** | Logical surface model | No wlroots; no Wayland session |
| **Local Model** | Python llama.cpp + stub mode | Not in initramfs; not wired to agent |

---

## North-Star Definition

A **fully agentic OS session** means:

1. User boots the ISO on bare metal or VM and sees a **minimal compositor-driven UI** (not a conventional DE).
2. The **Agent Core** is the sole author of visible UI structure; there is no fixed app launcher.
3. Every user action and system event flows through **MCP** — auditable, policy-gated, replayable.
4. On capability miss, the agent **synthesizes** a sandboxed lambda, **hot-registers** MCP routes, and **materializes** widgets bound to those routes.
5. Over time, the agent's involvement **shrinks** as routes retire to direct lambda/UI binding.
6. **Proactive behavior** (timers, D-Bus, fs events, heartbeat snapshots) wakes the agent without user input.
7. **Real-time paths** (input, audio, video frames) never block on inference.

---

## Phase 1 — LLM Integration (L4 Completion)

**Goal:** Replace heuristic classify/plan with real local + cloud inference.

### 1.1 Wire Local Model into Agent Core

- Add `local-model` to initramfs (or sidecar socket in hybrid dev mode).
- Agent `classify_intent` → `local-model.complete` with privacy-tagged prompts.
- Agent `plan` → structured JSON plan from model output (schema-validated).
- Fallback: keep heuristic path when `LOCAL_MODEL_PATH` unset (current stub behavior).

### 1.2 Cloud Router

- Implement Tier B routing per `agent-core-spec.md`: complexity gate, privacy tag, local-only mode.
- Add provenance binding: cloud calls must cite user-intent trace ID.
- Rate limit and cost accounting in State Store (`task.cloud_usage.*`).

### 1.3 Skills System

- Load skills from State Store (`agent.skills.*`) at boot.
- Version skills; apply `applies_to` filters per wake category.
- Ship built-in skills: intent-classification, media-control, calculator-synth, notification-triage.

**Deliverable:** Agent produces real plans from GGUF model on device; cloud optional.

---

## Phase 2 — Display Stack (L5 Completion)

**Goal:** Boot into a real graphical session.

### 2.1 wlroots Compositor

- Integrate `wlroots` in `compositor/` (dependency already in workspace `Cargo.toml`).
- Own `wl_display`, seat, output, xdg-shell surfaces.
- Map UI Runtime tree nodes → Wayland surfaces (via `compositor.surface` MCP).
- Input routing: pointer/keyboard → `ui.event` on hit-tested widget (no agent in path).

### 2.2 UI Runtime Renderer Bridge

- Connect Rust `ui-runtime` to compositor via shared damage region protocol.
- Port Python AUIL/ASL parser to Rust (or run Python ui-engine as MCP sidecar during transition).
- Implement ASL token resolution on GPU-friendly style sheet (current `resolve_token` is CPU-only).

### 2.3 Init Session

- Replace PID-1 sleep loop with: `compositor` as session leader, `WAYLAND_DISPLAY=wayland-0`.
- `ui-runtime` subscribes to `state.watch` on `ui.root` for live tree updates.
- `fallback-shell` on VT switch for recovery.

**Deliverable:** ISO boots to interactive graphical UI driven by agent-patched AUIL tree.

---

## Phase 3 — Policy & Persistence Hardening (L2 + L1)

**Goal:** Production-grade security and state survival across reboot.

### 3.1 Port Policy Broker to Rust

- Port `PolicyInterpreter` rule engine from Python.
- Wire confirmation daemon (human-in-the-loop CONFIRM/HOLD).
- Every MCP call through broker middleware (not just agent calls).

### 3.2 State Store on RocksDB (Rust)

- Port RocksDB backend from Python `state_store/rocksdb_backend.py`.
- Implement `state.watch` with broadcast subscriptions.
- Persist MCP registry to `perm.mcp_routes.*` (rebuild bus on boot from store).

### 3.3 Registry Lifecycle

- `_bus.deregister` on `lambda.deprecate` / crash-loop rollback.
- Broker validates `_bus.register` (currently trusted-component only).
- Audit log entry per registration (policy-broker-spec §7).

**Deliverable:** Reboot-safe state; broker is authoritative in boot path.

---

## Phase 4 — Event Fabric Expansion (Proactive OS)

**Goal:** OS reacts to the full Linux ecosystem, not just user typing.

### 4.1 D-Bus Adapter

- Subscribe to `org.freedesktop.Notifications`, `org.freedesktop.login1`, NetworkManager signals.
- Normalize to Event Bus `category`/`pattern`/`payload`.
- Configurable allow-list in Policy Broker (`CAP_DBUS_SUBSCRIBE`).

### 4.2 Filesystem Watcher

- `inotify`/`fanotify` adapter for paths declared in lambda manifests (`watches: [/home/user/Downloads]`).
- Coalesce rapid writes; emit `fs.change.*` events.

### 4.3 PipeWire / Audio Events

- Stream state changes (device hotplug, default sink change) → `audio.*` events.
- Never route audio buffers through agent; only **decision events**.

### 4.4 Rich Heartbeat

Extend current 30s heartbeat with:
- Running lambda inventory + health
- UI tree revision + focused widget
- Policy hold queue depth
- GPU/thermal snapshot from system-daemon

**Deliverable:** Agent acts on notifications, downloads, login events without user prompt.

---

## Phase 5 — Lambda Platform (L1 Scale-Up)

**Goal:** Agent-deployed code is safe, fast, and reusable at scale — **without OCI containers** (code is written, validated, and sandboxed directly).

### 5.1 Seccomp Sandbox Runtime (not OCI)

- Per-invocation namespaces + seccomp allowlist (existing `lambda-server/src/sandbox.rs`).
- Source synthesis writes Python/shell to `/var/the-machine/lambdas/` and registers entrypoint.
- Static validation pass before broker approval (`validate.rs`).

### 5.2 Warm Pools & Leases

- Pre-warm lease on `exposes_mcp` registration (`pool.rs`).
- `bus.lease` / `bus.lease.renew` for fast-path metadata (mcp-bus).

### 5.3 Code Synthesis Pipeline

- Agent emits function source → `lambda.register` with validation.
- Auto-generate `input_schema`/`output_schema` from code introspection.

### 5.4 Function Library

- Semantic search over registry (`lambda.search` with `localmodel.embed` embeddings).
- User-visible "installed capabilities" derived from `bus.list_routes`.

**Deliverable:** Agent can deploy real Python/shell tools that persist and evolve in seccomp sandboxes.

---

## Phase 6 — Full Linux Distribution

**Goal:** Move from initramfs demo to installable OS.

### 6.1 Root Filesystem

- Build Debian/arch-style rootfs with systemd (or s6) launching Machine services.
- Kernel: standard LTS + DRM/KMS + PipeWire + seatd.
- Installer ISO (live + persist).

### 6.2 Hardware Support

- GPU drivers (mesa, proprietary option).
- Wi-Fi firmware, Bluetooth, power management.
- Secure Boot signing pipeline.

### 6.3 Cloud Agent Burst

- Optional cloud lambda burst for heavy compute (video transcode, large model calls).
- Same MCP surface; broker tags provenance as `cloud-burst`.

**Deliverable:** Installable agent-native Linux distribution.

---

## Phase 7 — Ecosystem & Extensibility

**Goal:** Third-party and cross-machine agent capabilities.

### 7.1 External MCP Servers

- Allow-list external MCP servers (HTTP/stdio) as bus handlers.
- Broker scopes: which external tools agent may invoke.

### 7.2 MCP Tool Marketplace

- Signed capability bundles (lambda + AUIL widget pack + policy rules).
- User CONFIRM on install.

### 7.3 Multi-User / Multi-Session

- Per-user state namespaces in State Store.
- Separate agent sessions; shared system-op routes.

### 7.4 Observability

- OpenTelemetry export of MCP call graph.
- Replay/debug: re-run agent session from audit log + state snapshot.

---

## Recommended Implementation Order

```
Phase 1 (LLM) ──┬──> Phase 2 (Display) ──> Phase 6 (Distro)
                │
Phase 3 (Policy/Persist) ──> Phase 4 (Events) ──> Phase 5 (Lambda)
                                                      │
                                                      └──> Phase 7 (Ecosystem)
```

**Critical path to "demoable agentic desktop":** 1 → 2 → 3 (minimal)  
**Critical path to "daily-driver prototype":** + 4 + 5 + 6

---

## MCP Tool Surface Growth Model

The OS has **no fixed application set**. Capabilities grow through:

| Mechanism | MCP namespace | Owner |
|-----------|---------------|-------|
| Agent synthesizes function | `mcp-intent` | lambda-server |
| Agent registers event handler | `event-handler` | lambda-server + event-bus |
| OS ships fixed routes | `system-op`, `state-op` | boot image |
| External MCP server | `mcp-intent` (imported) | broker-approved |

New tools should be added by:
1. Implementing handler in appropriate component.
2. Registering route at boot (fixed) or via `_bus.register` (dynamic).
3. Documenting in component spec + adding policy rules.
4. Adding integration test in `tests/integration/`.

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Boot to interactive UI | < 15s on VM |
| Agent wake → visible UI change | < 3s (local model, warm) |
| MCP resolve latency p99 | < 1ms |
| Input event → widget binding (no agent) | < 16ms |
| Policy check latency p99 | < 5ms |
| Lambda cold start | < 500ms (interpreted) |
| Registry routes after 1 week simulated use | Agent wake rate drops 50%+ |

---

## Immediate Next Steps (Sprint-Sized)

1. **Wire `local-model` into agent-core** — replace `classify()` heuristic.
2. **wlroots proof-of-concept** — one `xdg_toplevel` showing `ui.root` text node.
3. **Persist MCP registry** — `state.set perm.mcp_routes` on `_bus.register`.
4. **D-Bus notifications adapter** — `org.freedesktop.Notifications` → event-bus.
5. **Integration test** — register `calc.*` → `bus.resolve("calc.add")` → invoke.

---

## Related Documents

- [Runtime Model](./runtime-model.md) — current wired architecture
- [MCP Bus Spec](../mcp-bus-spec.md) — registry and routing spec
- [Agent Core Spec](../agent-core-spec.md) — session loop and LLM tiers
- [Python ↔ Rust Overlap](../guides/python-rust-overlap.md) — dual implementation strategy
- [Expansion gaps tracker](./gap-analysis.md) — living gap checklist (maintained with code)

---

## Appendix: Technology Choices

| Concern | Recommendation | Rationale |
|---------|----------------|-----------|
| Inference | llama.cpp / GGUF | On-device privacy, no network required |
| Protocol | MCP over Unix sockets | Already universal in codebase |
| Sandbox | seccomp namespaces (code synthesis) | No OCI — agent writes code, sandbox executes |
| Compositor | wlroots | Mature, fits minimal-DE goal |
| UI language | AUIL + ASL | Agent-friendly declarative trees |
| State | RocksDB | Embedded, fast patch semantics |
| Init | Custom → systemd | Initramfs now; systemd for rootfs |
| Cloud LLM | OpenAI-compatible API | Swappable; broker-gated |

This proposal is intentionally expansive. Each phase can be scoped into individual PRs against `main` using the existing CI matrix (Rust binaries, Python wheels, ISO).
