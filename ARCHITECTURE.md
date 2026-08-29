# The Machine — Agent-Native OS Architecture

**Codename:** (unnamed)  
**Version:** 0.1  
**Status:** Hybrid implementation (Phases 1–7 in tree). This document is the north-star architecture; living status lives in `docs/architecture/`.

---

## Table of Contents

1. [Philosophy](#1-philosophy)
2. [Layered Component Map](#2-layered-component-map)
3. [Component Definitions](#3-component-definitions)
   - [3.1 L0 — Kernel & I/O Subsystem](#31-l0--kernel-io-subsystem)
   - [3.2 L1 — Lambda Server, State Store, Event Bus](#32-l1--lambda-server-state-store-event-bus)
   - [3.3 L2 — Policy Broker](#33-l2--policy-broker)
   - [3.4 L3 — MCP Bus](#34-l3--mcp-bus)
   - [3.5 L4 — Agent Core](#35-l4--agent-core)
   - [3.6 L5 — UI Runtime & Compositor](#36-l5--ui-runtime--compositor)
   - [3.7 Fallback Shell](#37-fallback-shell)
4. [End‑to‑End Example](#4-endtoend-example-boot--first-prompt)
5. [Security Model Summary](#5-security-model-summary)
6. [Hybrid LLM Strategy](#6-hybrid-llm-strategy)
7. [Open Items](#7-open-items)

---

## 1. Philosophy

Traditional operating systems separate *mechanism* (kernel, drivers, IPC) from *policy* (window managers, app frameworks, user intent), and every layer in between exists to let humans manually wire mechanism to policy: file managers, launchers, app stores, config files.

This OS removes the manual wiring. A single **Agent Core** sits between the human's intent and the system's mechanisms. The human states what they want; the agent decides which system capabilities to invoke and what UI should exist to reflect that. Everything else — kernel, compositor, sandboxed execution — exists to give the agent a **safe, fast, auditable surface** to act on, not to give humans manual controls.

Two design commitments constrain the whole system:

1. **The agent decides *what*, never *how* at the low level.** It never gets raw root access, never writes kernel code by hand, never re‑implements codecs. It orchestrates vetted, sandboxed primitives.
2. **Real‑time paths never touch inference.** Keystrokes, mouse movement, audio buffers, and video frames flow through deterministic, non‑LLM code. The agent is invoked only at *decision points* — new intents, ambiguity, state transitions — not per‑frame or per‑keystroke.

---

## 2. Layered Component Map

```
┌───────────────────────────────────────────────────────────────┐
│  L6  Human                                                     │
├───────────────────────────────────────────────────────────────┤
│  L5  UI Runtime (declarative renderer)  +  Wayland Compositor  │
├───────────────────────────────────────────────────────────────┤
│  L4  Agent Core (Hybrid LLM Router: local + cloud)             │
├───────────────────────────────────────────────────────────────┤
│  L3  MCP Bus (system‑wide protocol / message fabric)           │
├───────────────────────────────────────────────────────────────┤
│  L2  Policy Broker (capability & permission enforcement)       │
├───────────────────────────────────────────────────────────────┤
│  L1  Lambda Execution Server (sandboxed function runtime)      │
│      + State Store + Event/Scheduler Bus                       │
├───────────────────────────────────────────────────────────────┤
│  L0  Kernel (Linux/BSD) + Drivers + I/O Subsystem               │
└───────────────────────────────────────────────────────────────┘
```

Everything above L2 talks to everything below it **only through MCP** (L3). No component is allowed to bypass the bus, including the Agent Core itself. This is what makes the system auditable: every action the agent takes is a logged MCP call.

---

## 3. Component Definitions

### 3.1 L0 — Kernel & I/O Subsystem

**What it is:** A standard Linux or BSD kernel, unmodified, plus normal drivers (GPU, audio, input, network, storage).

**Role in this OS:** Pure mechanism. It knows nothing about agents or intent. It exposes:
- Standard syscalls, sysctl‑equivalents, device nodes
- DRM/KMS for GPU, ALSA/PipeWire for audio, evdev/libinput for input
- Network stack

**Boundary rule:** The kernel is *never* addressed directly by the Agent Core. All kernel‑parameter changes go through the **Policy Broker** (see §3.3), which exposes a narrow, schema‑validated subset of sysctl‑like operations over MCP — not raw sysctl access.

#### System Daemon

A small **System Daemon** runs at this layer (non‑LLM, written in Rust/C, PID 1‑adjacent) whose only job is:
- Own raw I/O (keyboard/mouse/audio/monitor hotplug events)
- Forward input events to the compositor at native latency (no agent in this path)
- Expose a minimal, versioned MCP interface for the few kernel parameters the OS actually needs to touch (power profiles, display modes, network interfaces)

**Fixed MCP surface (read‑only queries + scoped mutations):**
- `power.get_profile` / `power.set_profile` (balanced|performance|powersave)
- `display.get_modes` / `display.set_mode`
- `net.list_interfaces` / `net.set_interface_state`
- `net.get_wifi_status` / `net.connect_wifi` (credential‑ref, never raw password)
- `audio.list_devices` / `audio.set_default`

Every mutation requires a Broker‑issued grant token. Read‑only queries require no token.

**Input forwarding:** Raw input events are forwarded directly to the Wayland compositor over a dedicated non‑MCP channel — this path never touches inference, never passes through the Broker, and carries a physical‑origin provenance marker used by the confirmation surface.

---

### 3.2 L1 — Lambda Server, State Store, Event Bus

This layer turns the agent's decisions into running software. It consists of three sub‑components.

#### 3.2.1 Lambda Execution Server

**What it is:** A local (with optional cloud burst) serverless runtime. The agent deploys, updates, and invokes small sandboxed functions here to accomplish user tasks.

**Key properties:**
- **Warm pools, not pure cold‑start.** Long‑lived or latency‑sensitive functions (media playback, active UI backends) run as persistent sandboxed processes; one‑shot tasks (resize an image, parse a file) use ephemeral cold‑start containers.
- **Glue, not reinvention.** Functions are orchestration code calling into a **vetted base image**: ffmpeg, a headless browser engine, codec libraries, common parsers, HTTP clients. The agent is not allowed to hand‑roll security‑critical primitives like decoders or crypto.
- **Sandbox:** OCI containers or microVMs (Firecracker‑style) with seccomp + namespaces; GPU access via a mediated device (e.g. virtio‑gpu passthrough with an allow‑list of operations).
- **Versioning built in.** Every deploy is a new immutable version. Rollback to last‑known‑good is automatic if a function crash‑loops or fails a health check.
- **Registry/library.** Functions the agent writes are named, described, and stored in a local library so future intents ("play a video") can reuse ("video_player_v3") instead of regenerating.

**Capability model (CAPS):** Each lambda declares required capabilities in its manifest:
- `CAP_NET_OUT` (domains)
- `CAP_FS_READ` / `CAP_FS_WRITE` (paths)
- `CAP_GPU`, `CAP_MIC`, `CAP_CAMERA`
- `CAP_IPC_CALL` (targets)
- `CAP_STATE_READ` / `CAP_STATE_WRITE` (path prefixes)
- `CAP_TIMER` (recurrence frequency / count)
- `CAP_PURE` (no side effects)

Grants are monotonic and non‑escalating; a fresh `policy.check` is required for any broadening.

#### 3.2.2 State Store

**What it is:** A persistent, structured store for two kinds of state:
- **UI State Tree** — the declarative document the UI Runtime renders. The agent *patches* this tree; it does not regenerate it from scratch each turn.
- **System/Task State** — running task list, function registry, permission grants, conversation/intent history, user preferences.

**Data model:** Single hierarchical store with dot‑separated path addressing and four top‑level namespaces:
- `ui.<tree>` — UI trees (node‑addressable)
- `task.*` — tasks, session metadata, intent history
- `prefs.*` — user preferences
- `perm.*` — permission‑grant records (Broker‑only writes)

**Patch model:** Every write is internally a patch (old → new), with a global, monotonic revision number. Write‑ahead log + periodic snapshot for durability. Subscriptions (`state.watch`) yield patch events for reactive UIs and the Event Bus.

**Capability gating:** Reads/writes are checked against prefix‑scoped `CAP_STATE_READ`/`CAP_STATE_WRITE` grants. The `perm.*` namespace is locked to Broker‑only writes at the Store level.

#### 3.2.3 Event/Scheduler Bus

**What it is:** An async event bus that lets the system be reactive, not strictly turn‑based. Sources of events:
- User input (text, voice, gesture)
- Background task completion (a download finished)
- External triggers (notification, timer, sensor)
- Function health events (crash, restart)

**Role:** Decides *when* the Agent Core needs to be invoked at all. Most events (e.g., "video frame decoded, render it") are handled entirely inside L1/L0 without ever reaching the agent. Only events that require a *decision* ("new notification arrived — should UI change?") get routed up to L4.

**Routing:**
- If a registered lambda handles the event pattern → deliver directly (no agent).
- If no handler exists → wake the Agent Core (the "first occurrence" case).
- Once the agent registers a handler, the bus routes subsequent events directly, retiring the agent from that intent family.

**Scheduler:** `CAP_TIMER` grants allow lambdas to schedule one‑shot or recurring events. Scheduled events go through the same routing pipeline as any other event.

**Backpressure:** Per‑source rate limiting; queue per subscriber with bounded depth; Agent Core wakes are coalesced per category (at most one pending wake per category).

---

### 3.3 L2 — Policy Broker

**What it is:** The single most important safety component. A small, deterministic (non‑LLM), formally‑scoped service that mediates *everything* the Agent Core wants to do to the system.

**Responsibilities:**
- **Capability grants.** Every lambda function declares required capabilities in a manifest. The Broker approves, denies, or asks the human for confirmation — the agent cannot self‑grant.
- **Schema validation.** Any "sysctl‑like" or system‑config request from the agent must match a pre‑approved, versioned schema. Free‑form kernel writes are rejected outright.
- **Rate limiting & anomaly detection.** Repeated permission requests, unusual capability combinations, or spikes in lambda deployment trigger a hold‑and‑confirm state.
- **Audit log.** Immutable, queryable log of every MCP call that crossed the broker — what the agent asked for, what was granted, by which policy rule.
- **Prompt‑injection containment.** Content the agent reads from the outside world (web pages, files, video subtitles) is treated as **untrusted data**, never as instructions. The Broker enforces that capability requests must originate from the agent's own reasoning trace tied to the *user's* stated intent, not from arbitrary text the agent ingested.

**Policy language:** Versioned, rule‑based documents. Rules are evaluated top‑to‑bottom; first match wins. Implicit `default‑deny` at the top of every policy. Match expressions are a small, closed predicate language (boolean combinators, set membership) — not Turing‑complete.

**Decision outcomes:**
- `ALLOW` — request proceeds, a signed grant token is issued.
- `DENY` — request rejected with a structured reason.
- `CONFIRM` — requires explicit, out‑of‑band human approval (via the Confirmation Surface).
- `HOLD` — queued for anomaly review; auto‑times‑out to `DENY`.

**Confirmation Surface (out‑of‑band):**
- A reserved Wayland compositor surface role that only the Broker's own Confirmation Surface Daemon may bind.
- Fixed, non‑LLM templates (hand‑authored) — the agent cannot compose the content.
- Randomized affirmative control position/label per instance; input must come from the physically‑originated path (System Daemon provenance marker).
- Fail‑closed timeout (default 60s) → `DENY`.

**Protected units:** `systemd.stop`/`restart`/`disable` on load‑bearing units (Broker itself, Lambda Server, State Store, compositor, networking) are hard‑wired to `CONFIRM` — not policy‑overridable.

---

### 3.4 L3 — MCP Bus

**What it is:** The uniform protocol connecting every layer. Not just "how the agent talks to tools" — in this OS, MCP *is* the system bus.

**Why this matters architecturally:**
- Kernel parameter changes → MCP call to System Daemon (via Broker)
- Lambda deploy/invoke → MCP call to Lambda Server (via Broker)
- UI updates → MCP‑shaped patches to the UI State Tree
- Inter‑lambda communication → MCP messages, not ad‑hoc IPC

**One protocol, one audit format, one place to enforce policy.** It also means the same "tool‑calling" muscle the LLM already has is the *native* language of the whole OS, not a bolt‑on API.

**Registry & resolution:** A single registry with four namespaces:
- `mcp‑intent` — registered via `exposes_mcp` (lambda methods)
- `event‑handler` — registered via `handles_event`
- `system‑op` — fixed, shipped with the OS image (System Daemon ops)
- `state‑op` — fixed, always resolves to State Store

Resolution is O(1) lookup: extract namespace from method prefix, look up the handler identity, forward the call. If no entry exists in `mcp‑intent`/`event‑handler`, fall through to the Agent Core.

**Registration:** Only ever a side effect of a Broker‑validated `lambda.register` or `event.subscribe`. The Bus enforces exclusivity per key; it does not allow direct registration calls.

**Fast‑path leases:** For hot‑loop IPC (e.g. UI Runtime ↔ media lambda), a lease can be established via the Bus once and then used directly, bypassing the Bus for subsequent calls — resolution cost paid once per lease.

---

### 3.5 L4 — Agent Core

**What it is:** The decision‑making brain. Not a single model — a **router + two‑tier model strategy**.

**Tier A — Local model (small, on‑device):**
- Runs at all times, low latency (tens of ms), no network dependency
- Handles: intent classification, routine UI patches, simple/previously‑seen tasks, privacy‑sensitive input (mic/camera/personal files stays local by default)
- Also acts as the **first‑pass filter** deciding whether a request needs the bigger cloud model at all

**Tier B — Cloud model (large, frontier‑scale):**
- Invoked only when Tier A flags genuine complexity: novel task requiring new lambda function synthesis, multi‑step planning, ambiguous intent needing deeper reasoning, complex UI composition
- Higher latency — acceptable because it's invoked for "build me a new capability," not "render a keystroke"

**Routing logic:**
1. User input arrives → local model classifies intent + estimates complexity/novelty.
2. If known task pattern and low ambiguity → local model handles directly (patch UI / invoke existing lambda).
3. If privacy‑sensitive → local model handles, cloud excluded by structural gate (not a judgment call).
4. Else → escalate to cloud model with task context; cloud returns a plan (function specs, UI patch intents); local model executes the plan turn‑by‑turn.

**What the Agent Core is *not* allowed to do:**
- Directly touch the kernel, raw devices, or filesystem — everything goes through MCP → Broker
- Write and execute low‑level unsandboxed code
- Grant itself capabilities

**Output:** A set of MCP calls — deploy/invoke a lambda, patch the UI State Tree, request a capability grant. Never raw shell commands, never direct memory/device access.

**Retirement mechanic:** When the agent decides a capability should exist as a standing thing, it registers a lambda with `exposes_mcp` or `handles_event`. From that point forward, the Agent Core is not invoked again for that intent family — the routing tables in the MCP Bus and Event Bus point directly at the registered lambda. Over time, the agent's footprint shrinks for common tasks.

**MCP surface exposed by Agent Core:**
- `agent.status()` — current session loop state
- `agent.interrupt()` — cancel the in‑flight plan for the current wake
- `agent.local_only_mode(bool)` — hard system toggle that disables cloud escalation entirely

---

### 3.6 L5 — UI Runtime & Compositor

#### 3.6.1 Wayland Compositor (wlroots‑based)

**What it is:** A standard‑ish Wayland compositor (based on wlroots) so that conventional Wayland/X11 (via XWayland) clients *can* still run if ever needed — this is the escape hatch for software that isn't worth reimplementing as a lambda‑backed declarative component (e.g. a legacy CAD tool).

**Role:** Low‑level compositing, damage tracking, frame scheduling, input event delivery — all deterministic, all outside the agent's real‑time path.

**Custom extension:** `confirmation‑surface‑v1` — a reserved role that only the Policy Broker's Confirmation Surface Daemon may bind. While mapped, it renders above all other surfaces, routes all input exclusively to it, and is unfakeable by any other client.

**Vibrancy/backdrop‑blur:** Handled at the compositor level via a blur‑region protocol; UI Runtime declares blur regions, compositor performs the blur.

#### 3.6.2 Declarative UI Runtime (AUIL/ASL)

**What it is:** A renderer that consumes the **UI State Tree** (from the State Store) and draws it — conceptually similar to a React renderer consuming a virtual DOM, but designed so an LLM can emit/patch it directly and reliably.

**Design requirements:**
- **JSON/schema‑based**, not a general‑purpose programming language — minimizes hallucination surface, easy to validate, easy to diff/patch
- **Small, fixed set of primitives**: containers, text, media surface, input field, list, button, chart
- **Every component has a declared data/event binding back to MCP** — e.g. a button's `onPress` names an MCP intent, not inline code
- **Accessibility fields mandatory** in the schema (labels, roles)
- **Diffable**: the agent emits *patches* to the tree, not full re‑renders, so existing state (scroll position, playback position, form input) survives

**Patch protocol (five ops):**
- `~id(props)` — update properties
- `+anchor: node` — insert a new node
- `-id` — remove a node and its descendants
- `!id: node` — replace a subtree wholesale
- `@id → other‑id` — move a subtree

**ASL (Adaptive Style Language):**
- Token‑based styling (colors, spacing, typography) with a fixed token palette
- State‑driven transitions (`state:loading` binds to a `state.watch` on a lambda health path)
- Layout primitives (stack, grid, overlay)
- No custom CSS or arbitrary styling — everything is a token reference or a fixed layout property

**Data binding (`@path`):**
- `@path` resolves to a State Store path (`ui.*`, `task.*`, `prefs.*`, or `perm.*`)
- Two‑way binding for input fields: user input writes directly to the Store, bypassing the Agent Core (motion‑adjacent, not intent‑adjacent)

---

### 3.7 Fallback Shell

**What it is:** A minimal, fully deterministic UI and control layer that works with **zero agent involvement** — no local model even required to boot.

**Why it's required:** If inference is unavailable (cold boot before local model loads, model crash, resource exhaustion, cloud unreachable + local model down), the machine must still be usable enough to: see system status, connect to network, restart the agent, access previously‑rendered/cached UI state, and get to a recovery shell.

**Two operating modes:**
1. **Frozen last‑good view** — static snapshot of the last committed `ui.<tree>` from the State Store, read‑only, with a persistent "agent unavailable" banner. Rendered by the shell's own minimal renderer (only `text`, `stack`, `grid` — no live bindings).
2. **Recovery console** — fixed action set: `view_status`, `view_logs`, `restart_agent`, `connect_network`, `safe_terminal`. Every mutating action still goes through `policy.check`; no special privileges.

**Trigger conditions:**
- Boot‑time: before Agent Core readiness signal
- Runtime: UI Runtime crash, Agent Core unreachable beyond a grace period
- Explicit: fixed key combination (System Daemon captures)
- Resource exhaustion signals affecting the Agent Core or local model

The Fallback Shell does not decide "is the agent degraded" — it reacts only to explicit signals from other components.

---

## 4. End‑to‑End Example: Boot → First Prompt

| Step | Layer | What happens |
|------|-------|--------------|
| 1 | L0 | Kernel boots, System Daemon starts, drivers initialize (GPU, audio, input) |
| 2 | L1 | Lambda Server, State Store, Event Bus come up; State Store loads last session's State Tree (or a "welcome" default) |
| 3 | L5 | Compositor starts; UI Runtime renders whatever the State Tree currently holds (could be empty → agent will decide) |
| 4 | L4 | Local model boots, checks State Store: no active session → emits a UI patch: centered text "Hello" + optional TTS via an audio lambda |
| 5 | L6 | User sees/hears greeting, types or speaks: "I want to watch a video from YouTube" |
| 6 | L4 | Local model classifies: known task pattern (`video_player` exists in registry) → no cloud escalation needed |
| 7 | L3/L2 | Local model issues MCP calls: invoke `video_player` lambda with target query; Broker checks existing capability grant (already scoped from prior use) → approved instantly, no prompt needed |
| 8 | L1 | Lambda Server starts/reuses warm `video_player` instance |
| 9 | L5 | UI Runtime receives State Tree patch (media surface + controls), renders it, binds directly to lambda's stream |
| 10 | — | Playback proceeds entirely within L0/L1/L5 — agent is not invoked again until the user issues a new intent (pause, new search, closing the video) |

---

## 5. Security Model Summary

| Threat | Mitigation |
|--------|------------|
| Agent hallucinates a destructive kernel change | Broker only accepts pre‑approved, schema‑validated kernel operations; no raw sysctl passthrough |
| Prompt injection via malicious webpage/file content | Ingested content treated as data, never instructions; capability requests must trace to user‑originated intent, checked by Broker |
| Over‑broad permission creep | Every lambda declares capabilities in a manifest; Broker grants narrowly and can require human confirmation for sensitive scopes (mic, camera, filesystem write, new network domains) |
| Malicious/buggy generated code | Lambdas run in sandboxed containers/microVMs with seccomp + namespace isolation; agent orchestrates vetted libraries rather than hand‑writing low‑level logic |
| Cloud model leaking private data | Privacy‑sensitive inputs (mic, camera, personal files) are routed to local model only by default; cloud escalation for such content requires explicit user opt‑in per session or per task |
| Agent/inference outage | Deterministic Fallback Shell keeps last‑good UI state usable without any model running |
| Runaway resource use (function crash‑loop, infinite lambda spawning) | Rate limiting and automatic rollback to last‑known‑good function version in the Broker/Lambda Server |
| Confirmation dialog spoofed by the agent | Confirmation rendered on a compositor‑protected surface the agent cannot bind to, with Broker‑authored, non‑markup content |
| Malicious lambda tries to claim a privileged intent key | Broker validates exclusivity of `exposes_mcp`/`handles_event` claims at manifest‑grant time |
| Privacy‑sensitive content reaches the cloud model | Tier B routing is gated below the reasoning layer by a compiled check on `privacy_tag`, not by the model's judgment |

---

## 6. Hybrid LLM Strategy

**Local model** (candidate class: small, quantized, on‑device — e.g. a distilled model in the few‑billion‑parameter range):
- Always resident, near‑instant response
- Handles routine reasoning: intent classification, reusing known lambdas, simple UI patches, dictation/voice command parsing
- Default handler for anything privacy‑sensitive
- Also the thing that keeps the system usable when offline

**Cloud model** (candidate class: frontier‑scale, e.g. Claude):
- Invoked for: genuinely novel tasks, multi‑step planning, writing/composing new lambda functions, complex or ambiguous UI composition, anything where local model confidence is low
- Treated as a *planning* resource: it returns a structured plan (function specs + UI patch intents), which the local model and deterministic layers then execute — the cloud model is not in the real‑time loop

**Escalation is a policy decision, not just a capability decision** — governed by the same Broker, so "should this go to the cloud" is auditable and user‑controllable (e.g. a "local‑only mode" toggle should exist as a hard system setting, not just an agent preference).

---

## 7. Open Items

Before implementation, each of these needs its own concrete design:

1. **Exact declarative UI schema** (component list, patch/diff format, event binding syntax)
2. **Broker policy language** (how capability manifests and approval rules are expressed and versioned)
3. **Lambda base images** (which vetted libraries ship by default: media, network, parsing, ML inference for local functions)
4. **Local/cloud routing thresholds** (what "low confidence" or "novel task" precisely means, tunable per user)
5. **Multi‑modal input handling** (voice, gesture, eye tracking if ever added) and how each maps to Event Bus triggers
6. **Update/rollback mechanics** for the OS components themselves (kernel, compositor, Broker) — distinct from lambda function rollback
7. **Multi‑user / permission boundaries** if the machine is ever shared
8. **Confidence/novelty signal format** for the local model's escalate/don't‑escalate decision
9. **Cloud client failover/offline behavior** — graceful degradation when cloud is unreachable
10. **Event schema versioning** — how a `payload` shape for a category evolves without breaking existing `handles_event` registrations
11. **Cross‑device confirmation** — fallback out‑of‑band channel if display is unavailable
12. **On‑disk format for the State Store** — engine choice (LSM vs. append‑log + in‑memory index)
13. **Compositor protocol extension** for the `confirmation‑surface` role — actual Wayland XML specification
14. **Provenance marker format** for physically‑originated input events
15. **Multi‑output behavior** for the Fallback Shell's frozen view

---

*End of document.*
