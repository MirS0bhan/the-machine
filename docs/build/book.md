# The Machine — Architecture & Implementation Guide

*Agent-native operating system: an architecture for intelligent orchestration, policy
enforcement, and declarative UI, with the design now backed by runnable, tested components.*

---

## 0. About this document

This is the **expanded system document** for *The Machine*. It pulls together every
design spec in `docs/` and the `*/docs/spec.md` component specs, and — unlike the
original pre-implementation drafts — grounds each component in the **actual source code
that now exists** in this repository.

The document is organized as a book:

1. **Architecture definition** (Chapter 2) — the agent-native thesis, the L0–L6 layer
   model, the MCP-everything rule, and the security posture.
2. **Per-component chapters** — each design spec followed by an *auto-generated
   implementation reference* that inventories the real modules, classes, functions, and
   tests discovered in the source tree at build time.
3. **Supporting specs** — the MCP Bus, System Daemon, Fallback Shell, and Compositor,
   which are still design drafts awaiting implementation.

> The implementation references are produced by `docs/build.py` by walking the source
> with the `ast` module. They are regenerated on every `make docs` run, so this book
> never drifts silently from the code.

---

## 1. How to build, run, and test

### Building this documentation

```bash
make docs          # assemble specs + code references -> docs/build/index.html
make serve         # serve docs/build on http://localhost:8000
make clean         # remove docs/build
```

The build uses only the Python standard library plus the `markdown` package
(already present in the virtualenv). No `pandoc`/`mkdocs`/network access required.

### Running a component

> **Note:** The repo is migrating Python → Rust. Several components exist in both
> languages. See [Python ↔ Rust Overlap Guide](../guides/python-rust-overlap.md).

**Boot / ISO path (Rust daemons, Unix sockets):**

```bash
cargo run --bin mcp-bus
cargo run --bin system-daemon
cargo run --bin policy-broker      # full rule engine + confirmation UI (boot path)
cargo run --bin state-store        # sled or memory; STATE_STORE_PATH for persistence
cargo run --bin event-bus
cargo run --bin lambda-server
cargo run --bin local-model-daemon # GGUF or stub heuristics
cargo run --bin marketplace
cargo run --bin agent-core
cargo run --bin ui-runtime
```

**Dev / test path (Python reference servers, HTTP):**

```bash
pip install -e lambda-server policy-broker state-store local-model ui-engine event-bus
cd policy-broker && uvicorn policy_broker.mcp_server:app --port 8001
cd state-store    && STATE_STORE_BACKEND=memory uvicorn state_store.mcp_server:app --port 8002
cd lambda-server  && python3 test_server.py   # in-process tests
cd local-model    && python3 -m local_model.mcp_server
cd ui-engine      && python3 -m server
```

Or start everything: `THE_MACHINE_RUNTIME=hybrid ./scripts/start-services.sh`

### Running the tests

```bash
make test-all                    # Rust + Python (recommended)
make test-python                 # integration + component tests (mostly Python)
pytest tests/integration/ -v     # 30 cross-component tests (in-process Python)
```

The UI Engine demo (`ui-engine-demo/`) is the most complete end-to-end vertical:
an AUIL layout is parsed, patched through the real `patch_protocol`/`auil_parser`
code, rendered by a terminal `AbstractRenderer` implementation, and driven by a
keyboard input loop.

---

## 2. System at a glance

The Machine removes the manual wiring between *mechanism* (kernel, drivers, IPC) and
*policy* (window managers, app frameworks, user intent). A single **Agent Core** sits
between human intent and system mechanisms. The human states *what* they want; the
agent decides *which* system capabilities to invoke and what UI should exist. Everything
else — kernel, compositor, sandboxed execution — exists to give the agent a safe, fast,
auditable surface to act on.

Two design commitments constrain the whole system:

1. **The agent decides *what*, never *how* at the low level.** It never gets raw root
   access, never writes kernel code by hand, never re-implements codecs. It orchestrates
   vetted, sandboxed primitives.
2. **Real-time paths never touch inference.** Keystrokes, mouse movement, audio buffers,
   and video frames flow through deterministic, non-LLM code. The agent is invoked only
   at *decision points* — new intents, ambiguity, state transitions — not per-frame.

### The L0–L6 layer model

```
L6  Human
L5  UI Runtime (declarative renderer) + Wayland Compositor
L4  Agent Core (Hybrid LLM Router: local + cloud)
L3  MCP Bus (system-wide protocol / message fabric)
L2  Policy Broker (capability & permission enforcement)
L1  Lambda Execution Server + State Store + Event/Scheduler Bus
L0  Kernel (Linux/BSD) + Drivers + I/O Subsystem
```

**Everything above L2 talks to everything below it only through MCP (L3).** No component
is allowed to bypass the bus, including the Agent Core itself. This is what makes the
system auditable: every action the agent takes is a logged MCP call.

### What is implemented today

| Layer | Component | Status | Notes |
|---|---|---|---|
| L1 | Lambda Execution Server | **Implemented + tested** | HTTP + MCP API, `LocalExecutor`, capability enforcement, registry, supervisor |
| L1 | State Store | **Implemented** | MCP server, backend, capability-gated reads/writes, pub/sub |
| L1 | Event/Scheduler Bus | **Implemented** | Rust daemon (full); Python harness for integration tests |
| L2 | Policy Broker | **Implemented** | Rust rule engine (boot); Python reference for tests |
| L4 | Agent Core | **Implemented** | LLM planner via local-model-daemon + cloud router |
| L4 | Local Model Interface | **Implemented** | `local-model-daemon` (GGUF) + Python reference |
| L4 | Marketplace | **Implemented** | Curated bundle install |
| L5 | UI Engine | **Implemented** | AUIL/ASL parser, runtime, patch protocol, renderer, models |
| L5 | UI Engine Demo | **Implemented** | Terminal `AbstractRenderer`, `demo.auil`, input loop, tests |
| L3 | MCP Bus | **Implemented** | Dynamic intent registry, policy middleware, leases |
| L0 | System Daemon | **Implemented** | evdev input + mock kernel ops |
| L5 | Wayland Compositor | **Partial** | Framebuffer present loop; full wlroots (G16) planned |
| L3.7 | Fallback Shell | **Implemented** | Rust console recovery mode |

The architecture specs are carried forward in their chapters with live implementation
references appended where source code exists. See
[Runtime Model](../architecture/runtime-model.md) for the end-to-end agent→MCP→UI loop,
[Expansion Proposal](../architecture/expansion-proposal.md) for the roadmap to a fully agentic OS,
and [Python ↔ Rust Overlap Guide](../guides/python-rust-overlap.md) for dual-language components.

**Test coverage (run `make test-all`):**

| Suite | Tests |
|-------|-------|
| Integration (`tests/integration/`) | 30 |
| ui-engine | 10 |
| ui-engine-demo | 20 |
| policy-broker | 9 |
| state-store | 8 |
| local-model | 8 |
| lambda-server (`test_server.py`) | all sections |
| Rust workspace | `cargo test --workspace` |

---

## 3. Key architectural decisions (as built)

- **MCP is the only bus.** The Agent Core, Lambda Server, State Store, Event Bus,
  Policy Broker, and UI Engine all expose/consume MCP tools. The `mcp_interface` /
  `mcp_server` modules are the universal contract.
- **Capabilities, not trust.** Lambda manifests declare capabilities; the broker enforces
  them. This is already reflected in `lambda-server/enforcer.py` and the broker's
  decision engine.
- **State is incremental.** The State Store persists a UI State Tree that the agent
  *patches* — never regenerates — so scroll position, typed text, and playback survive
  between agent invocations.
- **UI is declarative & diffable.** AUIL (a line-oriented layout DSL) describes components;
  patches (`~id(props)`, `+path node`, `-id`) update the tree. The UI Runtime renders
  the tree through a pluggable `AbstractRenderer`, so the same logic drives a terminal
  demo today and a `wlroots` compositor tomorrow.
- **Local model owns the privacy boundary.** Sensitive input is tagged at the model
  layer; the Agent Core's MCP client mechanically refuses to route tagged context to the
  cloud model.

---

## 4. Security model (summary)

| Threat | Mitigation |
|---|---|
| Agent hallucinates a destructive kernel change | Broker only accepts pre-approved, schema-validated kernel operations |
| Prompt injection via malicious content | Ingested content is data, never instructions; capability requests must trace to user intent |
| Over-broad permission creep | Every lambda declares capabilities in a manifest; broker grants narrowly |
| Malicious/buggy generated code | Lambdas run sandboxed; agent orchestrates vetted libraries |
| Cloud model leaks private data | Privacy-tagged input is routed to local model only by a compiled gate |
| Agent/inference outage | Deterministic Fallback Shell keeps last-good UI state usable |
| Confirmation dialog spoofing | Confirmation rendered on a compositor-protected surface the agent cannot bind |

---

## 5. Project layout

```
the-machine/
├── docs/                     # documentation set + guides/python-rust-overlap.md
│   ├── spec.md               # architecture definition
│   ├── *-spec.md             # component design specs
│   ├── book/                 # expanded narrative + demo chapter
│   ├── build.py              # documentation generator
│   └── build/                # generated output (index.html, book.md)
├── agent-core/               # L4 hybrid LLM router (Rust)
├── lambda-server/            # L1 sandboxed runtime (Python + Rust)
├── state-store/              # L1 UI + system state (Python + Rust)
├── event-bus/                # L1 reactive routing (Python harness + Rust daemon)
├── policy-broker/            # L2 capability enforcement (Python + Rust)
├── mcp-bus/                  # L3 message fabric (Rust)
├── system-daemon/            # L0 I/O + kernel ops (Rust)
├── compositor/               # L5 Wayland compositor (Rust, partial)
├── fallback-shell/           # L5 recovery UI (Rust)
├── ui-runtime/               # L5 declarative renderer daemon (Rust)
├── local-model-daemon/       # L4 Tier-A inference (Rust, GGUF)
├── marketplace/              # L4 curated bundle install
├── local-model/              # L4 Tier-A inference (Python reference)
├── ui-engine/                # L5 AUIL/ASL parser (Python)
├── ui-engine-demo/           # L5 terminal demo app
├── build/                    # mkinitramfs.sh, mkiso.sh, CI packaging
├── scripts/                  # start-services.sh, verify-all.sh, verify-docs-code.*
└── Makefile                  # build, test, iso, ci-package
```

See the per-component chapters for the full module inventory and test counts.

---

## 6. Reading guide

- New to the system? Start with the **Architecture definition** chapter, then the
  **Lambda Execution Server** and **UI Engine** chapters — the two most complete verticals.
- Integrating a component? Jump to its chapter; the auto-generated *Implementation
  Reference* lists the exact modules, classes, and `test_*` count.
- Curious about safety? The **Policy Broker** chapter plus the Fallback Shell /
  Compositor / System Daemon specs cover the full trust model.

[TOC]

---

# Agent-Native OS — Architecture Definition
 
**Codename:** (unnamed)
**Version:** 0.1  
**Status:** Hybrid implementation — Rust boot daemons + Python reference servers (see [overlap guide](./guides/python-rust-overlap.md))
 
---
 
## 1. Philosophy
 
Traditional operating systems separate *mechanism* (kernel, drivers, IPC) from *policy* (window managers, app frameworks, user intent), and every layer in between exists to let humans manually wire mechanism to policy: file managers, launchers, app stores, config files.
 
This OS removes the manual wiring. A single **Agent Core** sits between the human's intent and the system's mechanisms. The human states what they want; the agent decides which system capabilities to invoke and what UI should exist to reflect that. Everything else — kernel, compositor, sandboxed execution — exists to give the agent a **safe, fast, auditable surface** to act on, not to give humans manual controls.
 
Two design commitments constrain the whole system:
 
1. **The agent decides *what*, never *how* at the low level.** It never gets raw root access, never writes kernel code by hand, never re-implements codecs. It orchestrates vetted, sandboxed primitives.
2. **Real-time paths never touch inference.** Keystrokes, mouse movement, audio buffers, and video frames flow through deterministic, non-LLM code. The agent is invoked only at *decision points* — new intents, ambiguity, state transitions — not per-frame or per-keystroke.
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
│  L3  MCP Bus (system-wide protocol / message fabric)           │
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
- Standard syscalls, sysctl-equivalents, device nodes
- DRM/KMS for GPU, ALSA/PipeWire for audio, evdev/libinput for input
- Network stack
**Boundary rule:** The kernel is *never* addressed directly by the Agent Core. All kernel-parameter changes go through the **Policy Broker** (3.4), which exposes a narrow, schema-validated subset of sysctl-like operations over MCP — not raw sysctl access.
 
**A small "System Daemon"** runs at this layer (non-LLM, written in Rust/C, PID 1-adjacent) whose only job is:
- Own raw I/O (keyboard/mouse/audio/monitor hotplug events)
- Forward input events to the compositor at native latency (no agent in this path)
- Expose a minimal, versioned MCP interface for the few kernel parameters the OS actually needs to touch (power profiles, display modes, network interfaces)
---
 
### 3.2 L1 — Lambda Execution Server, State Store, Event/Scheduler Bus
 
This is the layer where the agent's decisions become running software. Three sub-components:
 
#### 3.2.1 Lambda Execution Server
**What it is:** A local (with optional cloud burst) serverless runtime. The agent deploys, updates, and invokes small sandboxed functions here to accomplish user tasks.
 
**Key properties:**
- **Warm pools, not pure cold-start.** Long-lived or latency-sensitive functions (media playback, active UI backends) run as persistent sandboxed processes; one-shot tasks (resize an image, parse a file) use ephemeral cold-start containers.
- **Glue, not reinvention.** Functions are orchestration code calling into a **vetted base image**: ffmpeg, a headless browser engine, codec libraries, common parsers, HTTP clients. The agent is not allowed (by policy, and by lack of low-level model capability being trusted) to hand-roll security-critical primitives like decoders or crypto.
- **Sandbox technology:** OCI containers or microVMs (Firecracker-style) with seccomp + namespaces; GPU access via a mediated device (e.g. virtio-gpu passthrough with an allow-list of operations).
- **Versioning built in.** Every deploy is a new immutable version. Rollback to last-known-good is automatic if a function crash-loops or fails a health check.
- **Registry/library.** Functions the agent writes are named, described, and stored in a local library so future intents ("play a video") can reuse ("video_player_v3") instead of regenerating.
#### 3.2.2 State Store
**What it is:** A persistent, structured store for two kinds of state:
- **UI State Tree** — the declarative document the UI Runtime renders (see 3.5). The agent *patches* this tree; it does not regenerate it from scratch each turn.
- **System/Task State** — running task list, function registry, permission grants, conversation/intent history, user preferences.
**Why it matters:** Without this, scroll position, half-typed text, and playback position would be lost every time the agent is invoked again. This store is what makes agent output *incremental* rather than throwaway.
 
#### 3.2.3 Event/Scheduler Bus
**What it is:** An async event bus that lets the system be reactive, not strictly turn-based. Sources of events:
- User input (text, voice, gesture)
- Background task completion (a download finished)
- External triggers (notification, timer, sensor)
- Function health events (crash, restart)
**Role:** Decides *when* the Agent Core needs to be invoked at all. Most events (e.g., "video frame decoded, render it") are handled entirely inside L1/L0 without ever reaching the agent. Only events that require a *decision* ("new notification arrived — should UI change?") get routed up to L4.
 
---
 
### 3.3 L2 — Policy Broker
**What it is:** The single most important safety component. A small, deterministic (non-LLM), formally-scoped service that mediates *everything* the Agent Core wants to do to the system.
 
**Responsibilities:**
- **Capability grants.** Every lambda function declares required capabilities (network domains, filesystem paths, GPU/mic/camera access, kernel parameters) in a manifest. The Broker approves, denies, or asks the human for confirmation — the agent cannot self-grant.
- **Schema validation.** Any "sysctl-like" or system-config request from the agent must match a pre-approved, versioned schema. Free-form kernel writes are rejected outright.
- **Rate limiting & anomaly detection.** Repeated permission requests, unusual capability combinations (e.g. a "weather widget" function asking for filesystem write + camera), or spikes in lambda deployment trigger a hold-and-confirm state.
- **Audit log.** Immutable, queryable log of every MCP call that crossed the broker — what the agent asked for, what was granted, by which policy rule.
- **Prompt-injection containment.** Content the agent reads from the outside world (web pages, files, video subtitles) is treated as **untrusted data**, never as instructions. The Broker enforces that capability requests must originate from the agent's own reasoning trace tied to the *user's* stated intent, not from arbitrary text the agent ingested. (This is a policy/provenance check, not something an LLM self-polices.)
**Design stance:** The Broker is boring, deterministic, and heavily tested — the opposite personality of the agent. It is the load-bearing wall for the whole system's safety story.
 
---
 
### 3.4 L3 — MCP Bus
**What it is:** The uniform protocol connecting every layer. Not just "how the agent talks to tools" — in this OS, MCP *is* the system bus.
 
**Why this matters architecturally:**
- Kernel parameter changes → MCP call to System Daemon (via Broker)
- Lambda deploy/invoke → MCP call to Lambda Server (via Broker)
- UI updates → MCP-shaped patches to the UI State Tree
- Inter-lambda communication (e.g., video player function talking to a notes-taking function) → MCP messages, not ad-hoc IPC
**Benefit:** One protocol, one audit format, one place to enforce policy. It also means the same "tool-calling" muscle the LLM already has (from MCP-based agent training) is the *native* language of the whole OS, not a bolt-on API.
 
---
 
### 3.5 L4 — Agent Core (Hybrid LLM Router)
**What it is:** The decision-making brain. Not a single model — a **router + two-tier model strategy**, since you specified hybrid.
 
**Tier A — Local model (small, on-device):**
- Runs at all times, low latency (tens of ms), no network dependency
- Handles: intent classification ("is this a new task or a continuation?"), routine UI patches, simple/previously-seen tasks (reuse an existing lambda function), privacy-sensitive input (anything touching mic/camera/personal files stays local by default)
- Also acts as the **first-pass filter** deciding whether a request needs the bigger cloud model at all
**Tier B — Cloud model (large, frontier-scale):**
- Invoked only when Tier A flags genuine complexity: novel task requiring new lambda function synthesis, multi-step planning, ambiguous intent needing deeper reasoning, complex UI composition
- Higher latency (hundreds of ms–seconds) — acceptable because it's invoked for "build me a new capability," not "render a keystroke"
- Network-dependent; system must degrade gracefully if offline (see §6)
**Routing logic (example):**
```
User input arrives
  → Local model classifies intent + estimates complexity/novelty
  → If (known task pattern) AND (low ambiguity):
        Local model handles directly (patch UI tree / invoke existing lambda)
  → Else if (privacy-sensitive: mic/camera/personal file content):
        Local model handles, cloud model excluded regardless of complexity
        (or: user is prompted to explicitly allow cloud escalation)
  → Else:
        Escalate to cloud model with task context
        Cloud model returns a plan (which functions to deploy/call, UI shape)
        Local model executes the plan turn-by-turn afterward
```
 
**What the Agent Core is *not* allowed to do:**
- Directly touch the kernel, raw devices, or filesystem — everything goes through MCP → Broker
- Write and execute low-level unsandboxed code
- Grant itself capabilities
**Output of the Agent Core:** A set of MCP calls — deploy/invoke a lambda, patch the UI State Tree, request a capability grant. Never raw shell commands, never direct memory/device access.
 
---
 
### 3.6 L5 — UI Runtime & Wayland Compositor
 
#### 3.6.1 Wayland Compositor
**What it is:** A standard-ish Wayland compositor (could be based on wlroots) so that conventional Wayland/X11(via XWayland) clients *can* still run if ever needed — this is the escape hatch for software that isn't worth reimplementing as a lambda-backed declarative component (e.g. a legacy CAD tool).
 
**Role:** Low-level compositing, damage tracking, frame scheduling, input event delivery — all deterministic, all outside the agent's real-time path.
 
#### 3.6.2 Declarative UI Runtime
**What it is:** A renderer that consumes the **UI State Tree** (from the State Store, §3.2.2) and draws it — conceptually similar to how a React renderer consumes a virtual DOM, but designed so an LLM can emit/patch it directly and reliably.
 
**Design requirements for the language:**
- **JSON/schema-based**, not a general-purpose programming language — minimizes hallucination surface, is easy to validate, easy to diff/patch
- **Small, fixed set of primitives**: containers, text, media surface, input field, list, button, chart — composable, not infinitely extensible per-request
- **Every component has a declared data/event binding back to MCP** — e.g. a button's `onPress` names an MCP intent, not inline code
- **Accessibility fields mandatory** in the schema (labels, roles) so screen readers etc. work without special-casing
- **Diffable**: the agent (or local model) emits *patches* to the tree, not full re-renders, so existing state (scroll position, playback position, form input) survives
**Example flow:** "Play this YouTube video" →
1. Local model recognizes intent, checks function registry for `video_player`
2. If missing, escalate to cloud model → cloud model plans: deploy `video_player` lambda (ffmpeg + yt-dlp-equivalent glue, sandboxed, network scoped to video host + CDN), and emit a UI patch: add a `media_surface` component bound to that lambda's output stream, plus playback controls bound to its control intents
3. Broker validates and grants the network capability (scoped, e.g. to the specific domains needed)
4. Lambda Server spins up (or reuses warm pool), starts streaming
5. UI Runtime renders the patch; the media surface subscribes directly to the lambda's frame output over a low-latency local channel — **not through the agent** — for actual playback
---
 
### 3.7 Fallback / Degraded-Mode Shell
**What it is:** A minimal, fully deterministic UI and control layer that works with **zero agent involvement** — no local model even required to boot.
 
**Why it's required:** If inference is unavailable (cold boot before local model loads, model crash, resource exhaustion, cloud unreachable + local model down), the machine must still be usable enough to: see system status, connect to network, restart the agent, access previously-rendered/cached UI state, and get to a recovery shell.
 
**Behavior:** On agent failure, the UI Runtime keeps rendering the **last known-good State Tree** (from persistent State Store) frozen/read-only, with a visible "agent unavailable" indicator and basic recovery controls (restart agent, view logs, safe-mode terminal).
 
---
 
## 4. End-to-End Example: Boot → First Prompt
 
| Step | Layer | What happens |
|---|---|---|
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
|---|---|
| Agent hallucinates a destructive kernel change | Broker only accepts pre-approved, schema-validated kernel operations; no raw sysctl passthrough |
| Prompt injection via malicious webpage/file content | Ingested content treated as data, never instructions; capability requests must trace to user-originated intent, checked by Broker |
| Over-broad permission creep | Every lambda declares capabilities in a manifest; Broker grants narrowly and can require human confirmation for sensitive scopes (mic, camera, filesystem write, new network domains) |
| Malicious/buggy generated code | Lambdas run in sandboxed containers/microVMs with seccomp + namespace isolation; agent orchestrates vetted libraries rather than hand-writing low-level logic |
| Cloud model leaking private data | Privacy-sensitive inputs (mic, camera, personal files) are routed to local model only by default; cloud escalation for such content requires explicit user opt-in per session or per task |
| Agent/inference outage | Deterministic Fallback Shell keeps last-good UI state usable without any model running |
| Runaway resource use (function crash-loop, infinite lambda spawning) | Rate limiting and automatic rollback to last-known-good function version in the Broker/Lambda Server |
 
---
 
## 6. Hybrid LLM Strategy — Detail
 
**Local model** (candidate class: small, quantized, on-device — e.g. a distilled model in the few-billion-parameter range):
- Always resident, near-instant response
- Handles routine reasoning: intent classification, reusing known lambdas, simple UI patches, dictation/voice command parsing
- Default handler for anything privacy-sensitive
- Also the thing that keeps the system usable when offline
**Cloud model** (candidate class: frontier-scale, e.g. Claude):
- Invoked for: genuinely novel tasks, multi-step planning, writing/composing new lambda functions, complex or ambiguous UI composition, anything where local model confidence is low
- Treated as a *planning* resource: it returns a structured plan (function specs + UI patch intents), which the local model and deterministic layers then execute — the cloud model is not in the real-time loop
**Escalation is a policy decision, not just a capability decision** — governed by the same Broker, so "should this go to the cloud" is auditable and user-controllable (e.g. a "local-only mode" toggle should exist as a hard system setting, not just an agent preference).
 
---
 
## 7. What Still Needs Concrete Design Work
 
This document defines the *shape* of the system. Before implementation, each of these needs its own spec:
 
1. **Exact declarative UI schema** (component list, patch/diff format, event binding syntax)
2. **Broker policy language** (how capability manifests and approval rules are expressed and versioned)
3. **Lambda base images** (which vetted libraries ship by default: media, network, parsing, ML inference for local functions)
4. **Local/cloud routing thresholds** (what "low confidence" or "novel task" precisely means, tunable per user)
5. **Multi-modal input handling** (voice, gesture, eye tracking if ever added) and how each maps to Event Bus triggers
6. **Update/rollback mechanics** for the OS components themselves (kernel, compositor, Broker) — distinct from lambda function rollback
7. **Multi-user / permission boundaries** if the machine is ever shared
---
 
*End of document.*

---

# Lambda Execution Server — Function Registry, Process Isolation, IPC & MCP Control
 
**Fills:** §3.2.1 of `agent-native-os-architecture.md` (Lambda Execution Server) and §7.3 ("Lambda base images")
**Related:** `auil-asl-spec.md` §8 (MCP as a routing fabric) — this document is the server that §8 registers handlers *into*
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation
 
---
 
## 0. Design goals
 
1. **Functions are named, described, persistent, reusable.** The agent's job is to make a capability exist once, not to regenerate code every time a similar request comes in.
2. **Process is the trust boundary.** One function = one sandboxed process (or one warm-pool slot). Capability grants are attached to processes, not to code — code alone proves nothing to the sandbox.
3. **Cross-function calls are IPC, always** — never an in-process import, even for two functions in the same language sitting in the same warm pool. This is what makes the call graph inspectable and the capability model enforceable: if `x` calls `y`, that edge exists somewhere the Broker can see and gate, not buried in a language-level `import y`.
4. **Capabilities are a closed, versioned power set**, not free-form strings — same philosophy the parent doc applies to kernel operations (§3.3) and this doc applies one level down, to inter-function calls.
5. **The SDK is the only door.** A function's code never touches a raw socket. It calls `call("y", input)`; the framework decides whether that's a brokered round-trip or a leased fast-path channel, and refuses the call outright if the manifest didn't declare it.
6. **The server exposes itself over MCP**, so the agent's whole relationship to "write some code" becomes: search first, register once, never write it again.
 
---
 
## 1. Component map (expanding parent doc §3.2.1)
 
```
┌───────────────────────────────────────────────────────────────┐
│  Lambda Execution Server (L1)                                  │
│                                                                  │
│   ┌───────────────┐  ┌────────────────┐  ┌──────────────────┐  │
│   │ Function       │  │ Process         │  │ IPC Router /     │  │
│   │ Registry       │  │ Supervisor      │  │ Capability       │  │
│   │ (name, desc,   │  │ (spawn/kill,    │  │ Enforcer         │  │
│   │  schema, caps, │  │  warm pools,    │  │ (resolve target, │  │
│   │  version hist) │  │  cgroups)       │  │  check CAP_IPC,  │  │
│   └───────┬────────┘  └────────┬────────┘  │  issue leases)   │  │
│           │                    │             └────────┬────────┘ │
│           │                    │                       │          │
│   ┌───────▼────────────────────▼───────────────────────▼───────┐ │
│   │           Per-function sandboxed process pool                │ │
│   │   [x: python] ◄──IPC socket──► [y: python] ◄──► [z: go]      │ │
│   └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│   ┌──────────────────────────────────────────────────────────┐ │
│   │  MCP Control Interface (lambda.search / .register / ...)  │ │
│   └──────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────┘
                All of the above is itself one container
                (or one microVM), sitting under the L2 Broker,
                same as every other L1 component in the parent doc.
```
 
Every arrow that crosses a process boundary in this diagram is IPC. Every capability grant that lets an arrow exist was checked by the Broker before the process was ever spawned.
 
---
 
## 2. Capability model — the CAPS power set
 
### 2.1 The fixed set
 
Capabilities are a closed, versioned enum — a function's manifest declares a **subset** of this set (an element of its power set), never a free-form string. This mirrors the parent doc's stance on kernel operations: no capability the Broker doesn't already know how to validate.
 
```
CAP_NET_OUT(domains=[...])         — outbound network, scoped to named domains
CAP_NET_IN(port)                   — listen for inbound connections (rare; most functions don't need this)
CAP_FS_READ(paths=[...])
CAP_FS_WRITE(paths=[...])
CAP_MIC / CAP_CAMERA
CAP_GPU(scope=render|compute)
CAP_STATE_READ(paths=[...])        — State Store (§3.2.2 parent doc) read
CAP_STATE_WRITE(paths=[...])
CAP_IPC_CALL(targets=[name, ...])  — which OTHER functions this one may call
CAP_SPAWN_EPHEMERAL                — may ask the Supervisor for a throwaway sub-process
CAP_TIMER                          — may schedule itself via the Event/Scheduler Bus
CAP_SYS_PARAM(scope=[...])         — narrow, pre-approved sysctl-equivalents (rare; mirrors parent §3.3)
```
 
`CAP_IPC_CALL` is the one that matters most for this spec: it's a **declared call graph edge**, not a blanket "can do IPC" flag. If `x`'s manifest lists `CAP_IPC_CALL(targets=[y])` and `x`'s code tries to call `z`, the Enforcer rejects it before a socket is even opened — regardless of what `z`'s own permissions are.
 
### 2.2 Grant is monotonic, non-escalating
 
A function can never be granted more than its manifest declares, and the manifest can never expand without going back through the Broker (same as any other capability grant in the parent doc, §3.3). Two enforcement layers, deliberately redundant:
 
- **Kernel-level:** the Process Supervisor derives an actual seccomp filter + network namespace + mount namespace from the granted subset at spawn time. This is the layer that can't be lied to by buggy or malicious code.
- **SDK-level:** the per-language framework (§4) refuses to even attempt a `call()` to an undeclared target and raises immediately, so well-behaved code fails fast and loud instead of hitting a kernel wall silently.
 
### 2.3 Capability tiers as a shorthand (optional, for the Broker's UX)
 
To avoid the agent having to hand-author a capability list for every trivial function, the Registry can offer named presets that just expand to a fixed subset — `pure` (no caps at all — math, string processing, data transforms), `reader` (`CAP_STATE_READ` + `CAP_FS_READ` on a scoped path), `networked` (adds `CAP_NET_OUT` to a declared domain list). Presets are sugar; the Broker still validates the expanded set, not the preset name.
 
---
 
## 3. Process & warm pool model
 
- **One process per function**, isolated the same way the parent doc isolates lambdas generally (§3.2.1): OCI container or microVM, seccomp + namespaces, cgroup resource limits.
- **Warm vs cold**, same rule as the parent doc: frequently-hit or latency-sensitive functions (a calculator, a UI-bound handler) stay warm; rare one-shot functions cold-start per call.
- **Compiled languages pay their cost once.** Go/Rust functions are built in an ephemeral builder container on first `lambda.register`, and the compiled artifact — not the source — is what the Supervisor spawns from then on. Interpreted languages (Python/Ruby/R/JS) skip this step; the agent's iteration loop is faster for those, which is part of why they're the default for agent-authored glue.
 
---
 
## 4. IPC & the per-language SDK
 
### 4.1 The call, from the agent's/function's point of view
 
Given the scenario in the prompt — function `x` calls `y`, gets `output = lambda_server(y, input)` — the SDK makes that look like a normal function call, but every call is actually IPC underneath:
 
```python
# Python SDK
from lambda_sdk import call, state, capabilities
 
@capabilities(ipc_call=["y"])
def x(input):
    output = call("y", input)      # looks synchronous; is IPC under the hood
    return transform(output)
```
 
```javascript
// Node SDK
const { call, state, capabilities } = require("lambda-sdk");
 
capabilities({ ipcCall: ["y"] });
 
async function x(input) {
  const output = await call("y", input);   // Promise-wrapped IPC round trip
  return transform(output);
}
```
 
The same shape holds for Ruby (`LambdaSDK.call("y", input)`), R (`lambda_call("y", input)`), and Go/Rust (`sdk.Call(ctx, "y", input)` — typed, since compiled languages get compile-time schema checks against the target's declared `input_schema`/`output_schema` for free).
 
**Cross-language calls are normal.** Since the wire format is schema-typed bytes over a socket, not language-native objects, a JS function can call a Python function can call a Go function without either side knowing what the other is written in. The Registry's `input_schema`/`output_schema` (§6) is the actual contract; the language is an implementation detail of the callee.
 
### 4.2 Two call paths, one API
 
The SDK's `call()` doesn't force the caller to know or care which path is used — that's resolved underneath:
 
| Path | When | Mechanism |
|---|---|---|
| **Brokered call** | First call to a target in this process's lifetime, or target isn't warm | `call()` → IPC Router: checks `CAP_IPC_CALL` grant, resolves/spawns `y`, proxies input, returns output. Every brokered call is logged. |
| **Fast-path lease** | Repeat calls to the same target (e.g. a tight loop, or a UI-bound handler called every frame-adjacent tick) | Router hands back a **TTL-bound, capability-scoped socket lease** on first resolution. Subsequent `call()`s use the leased socket directly, process-to-process, no Router round-trip. Lease auto-expires and re-brokers periodically; Router can revoke it immediately if the manifest's grant changes. |
 
This gets you both things asked for: `output = lambda_server(y, input)` semantics for the common/first case, and direct IPC for the hot-loop case — without ever letting a function open an arbitrary socket itself. The lease is still something the Router issued and can kill; it's a fast lane, not a bypass.
 
### 4.3 State access
 
`state.get(path)` / `state.set(path, value)` in every SDK map to the parent doc's State Store (§3.2.2), gated by `CAP_STATE_READ`/`CAP_STATE_WRITE`. Same pattern as IPC: looks like a local call, is actually a scoped, capability-checked round trip.
 
---
 
## 5. Container & language toolchain
 
Rather than one bloated image with every interpreter installed, the container is **layered**: a minimal base (Supervisor, Router, Registry client, seccomp profiles) plus **per-language runtime layers** pulled in only when a function actually declares that runtime. Keeps cold-start images small and attack surface proportional to what's actually deployed.
 
| Language | Why it's in the default set |
|---|---|
| **Python** | Requested; also the deepest ecosystem for data/glue/ML work, and the language most agent-generated one-off functions will land in |
| **JavaScript/TypeScript (Node)** | Requested; natural fit given the rest of the OS already speaks JSON/MCP-shaped idioms, and it's the ecosystem most web-scraping/HTTP-glue code assumes |
| **Ruby** | Requested; strong for text/scripting-style glue, still the default in some domains (Rails-adjacent APIs, certain CLIs the agent might need to shell out to) |
| **R** | Requested; the natural backend for the `chart` AUIL primitive and any statistics-heavy function |
| **Go** *(suggested addition)* | Compiled, small static binaries, fast cold-start relative to a JVM-style runtime — good for latency-sensitive functions the agent promotes out of Python once they're proven hot (e.g. the IPC Router itself could be Go) |
| **Rust** *(suggested addition)* | For the rare function that needs to be a genuinely vetted, memory-safe primitive — the parent doc already says the agent shouldn't hand-roll crypto/decoders; Rust is the language those *vetted* base-image primitives are written in, and it's available if a function needs to link against one directly |
| **WASM runtime (wasmtime)** *(suggested addition)* | An extra containment layer, independent of source language: agent-generated code that's lower-trust (first-run, unreviewed, or from a less-trusted synthesis path) can be compiled to WASM and run under WASI's own capability model as a second sandbox inside the process sandbox — defense in depth for exactly the code the agent is least sure about |
| **Bash/POSIX shell** *(suggested addition, tightly capped)* | Thin orchestration/glue only — chaining existing vetted CLI tools (ffmpeg, etc., per parent doc §3.2.1). Should almost never be granted `CAP_FS_WRITE` or `CAP_NET_OUT` beyond what it's explicitly gluing together; treat as the lowest-trust-by-default runtime in the menu |
 
---
 
## 6. Function Registry — entry schema
 
```
function calc.add
  version: 3
  runtime: python3.12
  description: "Adds two or more numeric values, returns their sum."
  input_schema:  { values: number[] }
  output_schema: { sum: number }
  capabilities:  pure                         # expands to: (none)
  exposes_mcp:   calc.add                     # optional — registers as a direct MCP handler
                                                # per auil-asl-spec.md §8
  source: registry://calc/add/v3/main.py
  artifact: none                              # interpreted; no build step
  status: warm
  history: [v1 (2026-03-01), v2 (2026-04-11), v3 (2026-06-02, current)]
```
 
Compiled-language entries additionally carry `artifact:` pointing at the built binary and a `build_log:` reference. `exposes_mcp` is what lets a UI button's `on:press=mcp:calc.add` (AUIL) route straight to this function once it's registered, without the agent being invoked again — the mechanism defined in `auil-asl-spec.md` §8.
 
**Rollback** works exactly like the parent doc's lambda versioning generally (§3.2.1): every `lambda.register` on an existing name creates a new immutable version; the Supervisor auto-rolls-back to last-known-good on crash-loop or failed health check, same as any other lambda.
 
---
 
## 7. MCP control surface
 
This is the interface the Agent Core actually talks to — the Lambda Server is, from the Bus's point of view, just another MCP server:
 
| Tool | Purpose |
|---|---|
| `lambda.search(query)` | Semantic/keyword search over registry descriptions. Returns candidate `{name, description, input_schema, output_schema}` — this is the "is there already a function for this" step. |
| `lambda.describe(name)` | Full manifest for one function, including capability list and version history. |
| `lambda.register(name, runtime, code, description, input_schema, output_schema, capabilities, exposes_mcp?)` | Create or update a function. Triggers Broker capability validation → build (if compiled) → sandbox profile derivation → Registry entry. This is the "inject the function" step. |
| `lambda.invoke(name, input)` | Direct invocation — used when the agent (or another MCP client) wants a result immediately rather than through a UI-bound intent. |
| `lambda.deprecate(name, version)` / `lambda.rollback(name, version)` | Version lifecycle, mirrors parent doc §3.2.1. |
| `lambda.list_calls(name)` | Introspect a function's declared `CAP_IPC_CALL` graph — lets a human auditor or the Broker answer "what can this thing talk to" without reading the code. |
 
---
 
## 8. The workflow the prompt describes, end to end
 
> "Calculate something" →
 
1. Agent calls `lambda.search("calculate 47 * 12.5 with a running total")`.
2. **Hit:** a `calc.*` family already exists → `lambda.invoke("calc.eval", {...})` → done. No code written, no new process spawned if `calc.eval` is warm.
3. **Miss:** nothing matches → the agent, informed by a skill describing this Lambda Server's SDK conventions (§4), writes the function body in whichever runtime fits (Python, for a first-cut calculator) and its capability manifest (almost certainly `pure` — a calculator needs no network, filesystem, or state access).
4. Agent calls `lambda.register("calc.eval", "python3.12", <code>, <description>, <schemas>, capabilities="pure", exposes_mcp="calc.*")`.
5. Broker validates the manifest is a legal subset of the CAPS power set, Supervisor spawns the sandboxed process (or queues it warm), Registry stores it with the description that made it findable in step 1.
6. From here on: chat-driven calls hit it via `lambda.search` → `lambda.invoke` in one round trip with no code synthesis; UI-driven calls (a calculator button in AUIL) hit it directly via the `exposes_mcp` binding described in `auil-asl-spec.md` §8, without even the search step. Either way, the agent's inference is spent once, at creation time.
 
---
 
## 9. Security summary
 
| Threat | Mitigation |
|---|---|
| Function requests more than it needs | Manifest capabilities are a closed power set; Broker rejects anything outside the known enum, same posture as kernel ops in the parent doc |
| Function tries to call an undeclared target | `CAP_IPC_CALL(targets=[...])` is a declared call-graph edge; Enforcer rejects the call before a socket opens, independent of the callee's own permissions |
| Buggy/malicious agent-generated code | Process-per-function isolation (seccomp + namespaces + cgroups); crash-loop triggers automatic rollback to last-known-good version, same as parent doc §3.2.1 |
| Leaked or stolen fast-path lease | Leases are TTL-bound and capability-scoped; Router can revoke on manifest change; leases aren't raw unrestricted sockets, they're pre-authorized channels to one specific target |
| Supply-chain risk in language ecosystems (pip/npm/gem) | Vetted, versioned base images only; no arbitrary package installation at function runtime — same "glue, not reinvention" stance as the parent doc |
| Registry search results used as instructions rather than data | Search returns ranked metadata for the agent to *choose from*, never auto-invoked; `lambda.register` is the only path that creates a callable entry, and it goes through the same Broker validation as any other capability grant — an attacker can't get code executed just by getting a maliciously-described entry into search results, because search doesn't invoke |
| Compiled-artifact tampering between build and run | Registry stores an artifact hash alongside the binary path; Supervisor verifies hash at spawn |
 
---
 
## 10. Open items before implementation
 
1. **Wire format** for the IPC layer (length-prefixed msgpack is the likely default — cheap to parse in every target language, avoids JSON's per-call parsing overhead which matters at IPC volume even though it doesn't matter much at AUIL-authoring volume).
2. **`lambda.search` ranking** — pure keyword match is cheap but weak; embedding-based semantic search is better but adds a dependency the Lambda Server itself would have to run as... a lambda, which is a fun bootstrapping problem worth designing deliberately rather than accidentally.
3. **Resource quotas per capability tier** — CPU/memory/wall-clock limits should probably scale with what a function is trusted to do, not be flat across `pure` and `networked` functions alike.
4. **Capability power-set versioning** — how a new `CAP_*` gets added without invalidating every existing manifest that predates it (mirrors component-registry versioning in `auil-asl-spec.md` §9.3).
5. **Cross-function schema evolution** — if `calc.add`'s `output_schema` changes in v4, what happens to callers still declaring `CAP_IPC_CALL(targets=[calc.add])` against the v3 contract? Needs a compatibility policy, not just a version bump.
6. **WASM tier's relationship to the native sandbox** — is it a mode every function can opt into, or reserved specifically for lower-trust/first-run agent-generated code as suggested in §5? Worth deciding explicitly rather than letting it drift.
 
---
 
*End of document.*

### Implementation Reference (auto-generated)

Scanned `lambda-server`. Module / public-symbol inventory:

- **`bus_client.py`**
  - `def register_mcp_intent()`
  - `def deregister_mcp_intent()`
  - `def deregister_event_handler()`
  - `def register_event_handler()`
- **`config.py`**
  - `class ServerConfig`
    - `from_env()`
    - `setup_logging()`
- **`enforcer.py`**
  - `class EnforcementResult`
  - `class CapabilityEnforcer`
    - `validate_manifest()`
    - `expand_preset()`
    - `check_ipc_call()`
    - `check_state_access()`
    - `check_fs_access()`
    - `check_network_out()`
    - `register_process_grants()`
    - `get_process_grants()`
    - `revoke_process()`
  - `def create_grant()`
  - `def parse_capabilities()`
- **`example.py`**
  - `def main()`
- **`executor.py`**
  - `class ExecutionResult`
  - `class LocalExecutor`
    - `execute()`
- **`http_server.py`**
  - `class LambdaRequestHandler(BaseHTTPRequestHandler)`
    - `do_GET()`
    - `do_POST()`
    - `log_message()`
  - `def create_http_server()`
  - `def run_server()`
- **`mcp_interface.py`**
  - `class MCPControlInterface`
    - `handle_tool_call()`
    - `get_available_tools()`
- **`models.py`**
  - `class Capability(Enum)`
  - `class CapabilityPreset(Enum)`
  - `class CapabilityGrant`
    - `to_dict()`
  - `class FunctionManifest`
    - `source_hash()`
    - `to_dict()`
  - `class FunctionVersion`
  - `class IPCLease`
    - `is_expired()`
  - `class ProcessHandle`
    - `is_warm()`
- **`registry.py`**
  - `class FunctionRegistry`
    - `register()`
    - `get()`
    - `get_version_history()`
    - `search()`
    - `list_calls()`
    - `deprecate()`
    - `rollback()`
    - `resolve_mcp_pattern()`
    - `list_functions()`
- **`router.py`**
  - `class IPCCallResult`
  - `class IPCRouter`
    - `set_components()`
    - `brokered_call()`
    - `fast_path_call()`
    - `revoke_lease()`
    - `revoke_all_for_process()`
    - `revoke_all_for_target()`
    - `get_call_log()`
    - `get_active_leases()`
    - `cleanup_expired_leases()`
- **`sdk.py`**
  - `class LambdaContext`
    - `get_lease()`
    - `set_lease()`
  - `class StateAccessor`
    - `get()`
    - `set()`
  - `class LambdaFunction`
    - `to_dict()`
    - `execute()`
  - `def get_context()`
  - `def set_context()`
  - `def call()`
  - `def capabilities()`
  - `def get_capabilities()`
  - `def register_function()`
- **`server.py`**
  - `class LambdaServer`
    - `handle_mcp_tool()`
    - `get_tools()`
    - `health_check()`
  - `def create_server()`
- **`supervisor.py`**
  - `class WarmPoolConfig`
  - `class ProcessSupervisor`
    - `spawn()`
    - `kill()`
    - `get_warm()`
    - `add_to_warm_pool()`
    - `heartbeat()`
    - `check_health()`
    - `cleanup_stale()`
    - `get_process()`
    - `list_processes()`
    - `list_warm_pool()`
    - `get_stats()`

**Tests discovered:** 10 `test_*` functions.


---

# L1State Store Specification

## Purpose and Scope
The L1State Store is the single source of truth for the system's state. It provides persistent, consistent, and concurrent access to state data for all components.

## Key Responsibilities
- State persistence (disk/in-memory hybrid)
- Concurrency control (optimistic/pessimistic locking)
- State query and update APIs
- Event sourcing for state changes

## Dependencies
- **L1Event Bus**: For publishing state change events
- **L2Policy Broker**: For policy-aware state updates
- **L4Agent Core**: For agent-initiated state queries

## Interfaces
- **MCP Tools**: `state_get`, `state_update`, `state_query`
- **Events**: Publishes `state_updated`
- **CLI**: `state-cli` for state inspection

## Data Models
```python
class State:
    key: str
    value: Any
    version: int
    last_updated: datetime
```

## Open Questions
- What are the consistency requirements for distributed state?
- Should state be sharded or partitioned?
- How to handle state migration?

### Implementation Reference (auto-generated)

Scanned `state-store`. Module / public-symbol inventory:

- **`state_store/mcp_server.py`**
  - `def state_get()`
  - `def state_patch()`
  - `def state_watch()`
- **`state_store/memory_backend.py`**
  - `class MemoryBackend`
    - `get()`
    - `put()`
    - `delete()`
    - `patch()`
    - `list_paths()`
    - `create_snapshot()`
    - `release_snapshot()`
    - `close()`
- **`state_store/models.py`**
  - `class PatchOpType(str, Enum)`
  - `class PatchOp(BaseModel)`
  - `class WatchRequest(BaseModel)`
  - `class StateResponse(BaseModel)`
- **`state_store/policy.py`**
  - `def policy_check()`
- **`state_store/policy_client.py`**
  - `class PolicyClient`
    - `check()`
    - `close()`
- **`state_store/rocksdb_backend.py`**
  - `class RocksDBBackend`
    - `get()`
    - `put()`
    - `delete()`
    - `patch()`
    - `create_snapshot()`
    - `release_snapshot()`
    - `get_snapshot()`

**Tests discovered:** 11 `test_*` functions.


---

# Event/Scheduler Bus — Reactive Routing, Timers & Agent Wake Decisions

**Fills:** §3.2.3 of `agent-native-os-architecture.md` (Event/Scheduler Bus)
**Related:** `state-store-spec.md` §4 (`state.watch` as the underlying mechanism), `auil-asl-spec.md` §8 (MCP intent registry — the routing decision this bus makes is the same *kind* of decision), `lambda-server-spec.md` §2.1 (`CAP_TIMER`), `policy-broker-spec.md` §6 (rate limiting / anomaly interplay)
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation

---

## 0. Design goals

1. **Most events never reach the Agent Core.** This is the bus's entire reason for existing per parent §3.2.3: "video frame decoded, render it" must never wake an LLM. The default routing outcome for any event is *local resolution*, and waking the Agent Core is the exception that has to be earned by a routing rule, not the default.
2. **One mechanism, two jobs.** "Event bus" (react to things that happened) and "scheduler" (make things happen at a time) are the same component because both are, structurally, "notify a subscriber when a condition becomes true" — a timer is just an event whose trigger condition is a clock comparison instead of a state change.
3. **Routing is inspectable, not implicit.** Given the Broker's audit posture (`policy-broker-spec.md` §7) and the "agent retires from intent families" principle (`auil-asl-spec.md` §8), it must be possible to ask the bus *why* a given event class does or doesn't wake the agent, as a first-class query — not something you infer by reading logs after the fact.
4. **Built on the State Store's primitives, not parallel to them.** The bus does not maintain its own separate persistent event log; state changes are already durable and watchable (`state-store-spec.md` §4). The bus adds routing/scheduling semantics on top of `state.watch`, rather than re-implementing durability.

---

## 1. Event taxonomy

Events are typed, versioned, and closed-enum at the *category* level (so the routing table, §2, has a finite thing to switch on) but carry an open payload shape per category:

```
category: input          — user text/voice/gesture arriving at the UI Runtime
category: task-complete   — a lambda invocation or long-running task finished
category: health          — a lambda's health/status changed (crash, restart, degraded)
category: external        — notification, timer fired, sensor reading, network state change
category: state-change    — a raw state-store patch not already covered by the above
```

Every event carries: `category`, `source` (component/lambda identity), `payload` (category-specific shape), `timestamp`, and `state_revision` (the State Store revision, if any, this event corresponds to — lets a subscriber correlate an event with the exact state it should read).

---

## 2. Routing: local resolution vs. Agent Core wake vs. lambda intent handler

This is the bus's central responsibility, and it deliberately reuses the same three-way handler classification `auil-asl-spec.md` §8 already established for `mcp:` intents — because from the bus's point of view, "should this event wake the agent" and "should this button press wake the agent" are the same question asked from two different directions:

| Outcome | When | Mechanism |
|---|---|---|
| **Local resolution** | A registered subscriber (a lambda, or the UI Runtime itself) already handles this event category/pattern | Bus delivers directly to the subscriber via IPC (`lambda-server-spec.md` §4), no Agent Core involvement |
| **Lambda-hosted handler** | The event matches a pattern a lambda registered itself as the handler for (mirrors `exposes_mcp`, here `handles_event: <category>.<pattern>`) | Same routing table entry mechanism as `auil-asl-spec.md` §8 step 3–4, just keyed by event pattern instead of MCP intent name |
| **Agent Core wake** | No registered handler matches, or the event is explicitly flagged `requires_decision` by policy | Bus issues an MCP call to the Agent Core with the event; this is the only case that reaches inference |

**Routing table** is itself a Store-backed structure (`task.event_routes.*` in `state-store-spec.md`'s namespace), so registering a handler is a `state.set` call gated by the same capability model as everything else — a lambda declares `handles_event` in its manifest at `lambda.register` time, the Broker validates it same as any other capability claim (`policy-broker-spec.md` §11), and the bus's routing table gets an entry.

**First occurrence, concretely** (mirrors the calculator example in `auil-asl-spec.md` §8):
1. A download-complete event fires with no registered handler for `task-complete.download`.
2. Bus wakes the Agent Core with the event.
3. Agent Core decides what should happen (e.g. deploy a `download_notifier` lambda that shows a toast and logs completion) and, as part of that lambda's manifest, declares `handles_event: task-complete.download`.
4. Every subsequent `task-complete.download` event routes directly to `download_notifier`, bus → lambda, no agent involvement — same retirement mechanic as the MCP intent case.

---

## 3. Scheduler

- A lambda with `CAP_TIMER` (`lambda-server-spec.md` §2.1) may call `event.schedule(when, payload)` where `when` is either a one-shot timestamp or a recurrence rule (fixed cron-like grammar, not free-form).
- The scheduler is not a separate service from the event bus internally — a scheduled timer firing is just a `category: external, source: scheduler` event injected into the same routing pipeline (§2), so a scheduled wake goes through identical local-resolution-first logic as any other event; there's no special "timers always reach the agent" path.
- `CAP_TIMER` grants are scoped to a maximum recurrence frequency and a maximum number of concurrently scheduled timers per identity, enforced by the Broker at grant time — this is the anti-runaway-scheduling equivalent of the Lambda Server's rate limiting.

---

## 4. Subscription model

- `event.subscribe(category, pattern?)` — a lambda or the UI Runtime registers interest; delivery is push (IPC callback), not poll.
- Distinct from `handles_event` in a manifest: `subscribe` is "notify me, I'm not necessarily *the* handler" (multiple subscribers allowed, e.g. a logging lambda subscribing to everything for diagnostics); `handles_event` is "route this event *to* me as the authoritative handler" (one handler per pattern, exclusive, validated by the Broker to prevent two lambdas silently racing for the same event class).
- The UI Runtime's `state:*` ASL bindings (`auil-asl-spec.md` §3.5) do not go through `event.subscribe` at all — they use `state.watch` directly (`state-store-spec.md` §4), since that path is already real-time-safe and doesn't need routing/wake-decision logic layered on top. The bus's added value is specifically the wake-decision layer, which pure UI reactivity doesn't need.

---

## 5. MCP surface

| Tool | Purpose |
|---|---|
| `event.publish(category, payload)` | Inject an event (used by lambdas reporting task completion, health changes, etc.) |
| `event.subscribe(category, pattern?)` | Register a push subscription (§4) |
| `event.schedule(when, payload)` / `event.cancel(schedule_id)` | Timer management (§3) |
| `bus.explain_routing(category, pattern?)` | Introspection: returns the current routing outcome (local/lambda-handler/agent-wake) and, if a handler is registered, which one — the mechanism behind design goal §0.3 |
| `bus.list_handlers()` | Enumerate all `handles_event` registrations, for audit/debugging |

---

## 6. Backpressure & rate limiting

- Per-source publish rate limiting is enforced by the Broker (`policy-broker-spec.md` §6), keyed by publishing identity — the bus itself does not implement a separate limiter, to avoid two components disagreeing about what "too fast" means.
- When a handler (lambda or Agent Core) is slower than the event arrival rate, events queue per-subscriber with a bounded queue depth; on overflow, the bus drops the *oldest* queued event for that subscriber and increments a dropped-event counter surfaced via `bus.explain_routing` — silent unbounded queueing is treated as a bug, not a feature, since it would let one wedged handler consume unbounded memory.
- Agent Core wakes specifically are never queued more than one deep per event category — if the agent is already processing a wake for `category: health`, a second `health` event doesn't queue a second wake; it's coalesced, and the agent sees "at least one more health event occurred since your last wake" rather than replaying every intermediate event. This matches the parent doc's framing of the agent as a planning resource invoked at decision points, not a queue consumer.

---

## 7. Failure semantics

- A lambda health event (crash, crash-loop, restart) is itself routed through the same pipeline (§2) — most of the time this resolves to local handling (Process Supervisor rollback, per `lambda-server-spec.md` §3), and only escalates to an Agent Core wake if the Supervisor's own rollback fails or crash-loops persist past a policy-configured threshold.
- The bus's own failure (crash, restart) is a protected-unit condition at the Broker level (`policy-broker-spec.md` §5) — the bus is load-bearing enough that its own outage requires the same `CONFIRM`-gated restart handling as the Broker or State Store, not an ordinary lambda restart.

---

## 8. Security summary

| Threat | Mitigation |
|---|---|
| Lambda floods `event.publish` to force spurious agent wakes | Broker-enforced per-identity publish rate limiting (§6); wake coalescing per category (§6) |
| Two lambdas race to claim the same `handles_event` pattern | Broker validates exclusivity of `handles_event` claims at manifest-grant time, same enforcement point as capability grants (`policy-broker-spec.md` §11) |
| Malicious `event.schedule` used for persistence/backdoor timers | `CAP_TIMER` grants are frequency- and count-capped; scheduled events still route through the normal handler-resolution pipeline, so a scheduled event can't itself bypass capability checks on what it triggers |
| Bus outage silently drops safety-relevant events (e.g. crash notifications) | Bus restart is a Broker protected-unit action; dropped-event counters are queryable, not silent |

---

## 9. Open items before implementation

1. **Event schema versioning** — how a `payload` shape for a category evolves without breaking existing `handles_event` registrations (mirrors the schema-evolution open item in `lambda-server-spec.md` §10.5).
2. **Cron grammar** for `event.schedule` recurrence rules — needs to be specified precisely, not left as "cron-like."
3. **Coalescing granularity** (§6) — coalescing per category may be too coarse once there are many distinct event sources sharing a category; may need per-(category, source) coalescing instead.
4. **Cross-boot event durability** — do queued-but-undelivered events survive a bus restart, or is delivery best-effort within a boot session only? Ties into the State Store's WAL/snapshot cadence.
5. **Priority classes** — should a `health: crash` event be able to jump the queue ahead of a routine `state-change` event for the same subscriber, or is strict FIFO-per-subscriber sufficient?

### Implementation Reference (auto-generated)

Scanned `event-bus`. Module / public-symbol inventory:

- **`event_bus/mcp_server.py`**
  - `def handle_mcp()`
  - `def main()`
- **`event_bus/models.py`**
  - `class EventPublishRequest(BaseModel)`
  - `class EventRecord(BaseModel)`
- **`event_bus/router.py`**
  - `class EventRouter`
    - `register_handler()`
    - `publish()`
    - `publish_request()`
    - `list_published()`
    - `clear()`



---

# L2Policy Broker Specification

## Purpose and Scope
The L2Policy Broker is responsible for evaluating and enforcing policies that govern the behavior of the system. It acts as a gatekeeper for actions initiated by agents or external systems, ensuring compliance with predefined rules.

## Key Responsibilities
- Policy evaluation and enforcement
- Integration with L1State Store for context-aware decisions
- Event-driven policy triggers via L1Event Bus
- MCP tool surface for policy management

## Dependencies
- **L1State Store**: For state context during policy evaluation
- **L1Event Bus**: For event-driven policy triggers
- **L4Agent Core**: For agent-initiated policy checks

## Interfaces
- **MCP Tools**: `policy_evaluate`, `policy_manage`
- **Events**: Subscribes to `state_updated`, publishes `policy_decision`
- **CLI**: `policy-cli` for policy management

## Example Workflow
1. Agent requests action via MCP
2. Policy Broker evaluates request against current state
3. Broker publishes `policy_decision` event
4. System enforces decision

## Open Questions
- Should policies be versioned?
- How to handle policy conflicts?
- What are the performance requirements for policy evaluation?

### Implementation Reference (auto-generated)

Scanned `policy-broker`. Module / public-symbol inventory:

- **`policy_broker/audit.py`**
  - `class AuditLogger`
    - `log()`
    - `query()`
- **`policy_broker/client.py`**
  - `class PolicyClient`
    - `check()`
    - `close()`
- **`policy_broker/interpreter.py`**
  - `class PolicyInterpreter`
    - `register()`
    - `check()`
- **`policy_broker/mcp_server.py`**
  - `def load_default_policies()`
  - `def policy_check()`
  - `def policy_register()`
  - `def policy_confirm_result()`
  - `def policy_audit_query()`
- **`policy_broker/models.py`**
  - `class RateLimit(BaseModel)`
  - `class Rule(BaseModel)`
  - `class PolicyDoc(BaseModel)`
  - `class CheckRequest(BaseModel)`
  - `class CheckResponse(BaseModel)`
  - `class AuditEntry(BaseModel)`
- **`policy_broker/state_store.py`**
  - `class StateStoreClient`
    - `get_policy()`
    - `put_policy()`
    - `append_audit_log()`
    - `query_audit_log()`

**Tests discovered:** 9 `test_*` functions.


---

# Agent Core — Hybrid LLM Router, Session Loop & System Control Surface

**Fills:** §3.5 of `agent-native-os-architecture.md` (Agent Core) and part of §7.4 ("Local/cloud routing thresholds")
**Related:** `local-model-spec.md` (the Tier A client this doc's router calls into), `lambda-server-spec.md` §7 (`lambda.search`/`.register`), `auil-asl-spec.md` §8 (intent-registry retirement), `event-bus-spec.md` §2 (what wakes this component), `policy-broker-spec.md` §5 & §9 (systemd control, confirmation)
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation — supersedes any prior draft; no earlier version of this document exists in the project's current file set

---

## 0. Design goals

1. **A thin harness, not a framework.** The compiled Agent Core binary contains no task-specific branching — no `if intent == "play_video"` anywhere in Rust. Every piece of task intelligence lives in prompts and skills (§5) loaded at runtime. The harness's job is: run the session loop, hold two model clients, speak MCP, and enforce nothing itself beyond what the Broker already enforces on it as a capability-scoped component like any other.
2. **Scoped like a lambda, not privileged like a kernel.** The Agent Core has no special access path to anything. Every kernel/systemd/lambda/state/UI action it takes is an ordinary MCP call subject to `policy.check` (`policy-broker-spec.md` §4), exactly as if it were any other Broker-scoped component. Its authority is that other components are configured to route decisions to it, not that it holds elevated permissions.
3. **Retire early, retire often.** Per `auil-asl-spec.md` §8 and `event-bus-spec.md` §2, the Agent Core's steady-state job is to make itself unnecessary for a given intent family as fast as possible — synthesize once, register a deterministic handler, get out of the loop.
4. **Two models, one router, no hardcoded threshold table.** Per the parent doc's own framing (§6), local/cloud routing is Tier A's judgment call at runtime, not a static rule the harness enforces. The harness exposes both model clients uniformly; which one handles a given turn is a decision made *in* Tier A's reasoning, not *by* compiled logic gating access to Tier B.
5. **Privacy is a hard rule the harness can't be argued out of.** Unlike routing (a judgment call), the privacy boundary — sensitive content never reaches the cloud model — is enforced structurally (§4), not left to Tier A's discretion, because a judgment call that a compromised or simply wrong model could get wrong is not a boundary.

---

## 1. Component map

```
┌──────────────────────────────────────────────────────────────────┐
│  Agent Core (L4) — thin Rust harness                              │
│                                                                    │
│  ┌────────────────┐   ┌──────────────────┐   ┌──────────────────┐ │
│  │ Session Loop    │   │ MCP Client        │   │ Model Clients     │ │
│  │ (wake, gather   │◄─►│ (single point of  │◄─►│  - Local (Tier A) │ │
│  │  context, plan, │   │  contact with     │   │    via            │ │
│  │  emit MCP calls,│   │  every other      │   │    local-model-   │ │
│  │  sleep)         │   │  component)       │   │    spec.md         │ │
│  └────────┬────────┘   └──────────────────┘   │  - Cloud (Tier B)  │ │
│           │                                    │    frontier model  │ │
│  ┌────────▼────────┐                          └──────────────────┘ │
│  │ Skill/Prompt     │                                                │
│  │ Library (§5)     │                                                │
│  └─────────────────┘                                                │
└──────────────────────────────────────────────────────────────────┘
                All task intelligence lives here — nothing
                task-specific is compiled into the harness above.
```

The harness itself declares a capability manifest to the Broker like any lambda (§4) — it is not exempt from `lambda-server-spec.md`'s CAPS power set, it simply has a very broad `CAP_IPC_CALL` target list (it needs to be able to reach the Lambda Server, State Store, Event Bus, and Broker itself) because its *job* is orchestration, not because it's privileged.

---

## 2. Session loop & wake conditions

The loop is intentionally simple — the sophistication lives in the models it calls, not the loop shape:

```
loop:
  wake_reason ← await next wake signal        # from Event Bus (event-bus-spec.md §2:
                                                #   "no registered handler matched" or
                                                #   "requires_decision" flag), or a
                                                #   pending user input event
  context ← gather(wake_reason)                 # relevant state.get reads, recent
                                                #   intent history, lambda.search hits
                                                #   if the wake looks like a capability gap
  plan ← route_and_plan(context)                # §3 — Tier A first, Tier B if escalated
  for call in plan.mcp_calls:
      emit(call)                                # ui.patch / lambda.invoke / lambda.register /
                                                #   policy.check / event.subscribe / etc.
  sleep                                         # loop returns to await, does not poll
```

The Agent Core is never in the real-time path (parent Design Commitment #2) precisely because this loop only runs on a wake signal from the Event Bus — it has no independent per-frame or per-keystroke tick.

---

## 3. Hybrid routing (implementing parent §3.5 and §6)

Tier A (local model, always resident, see `local-model-spec.md`) runs first on every wake and does one of three things:

1. **Handle directly** — known task pattern, low ambiguity, no privacy escalation needed: Tier A emits the plan itself (a UI patch, a `lambda.invoke` against an existing registered function). No Tier B call.
2. **Handle locally by hard rule** — the wake context includes privacy-sensitive material (mic/camera/personal-file content, per the tag mechanism defined in `local-model-spec.md` §3). Tier A handles it regardless of confidence; Tier B is structurally excluded from this wake, not merely discouraged from it (§4).
3. **Escalate to Tier B** — novel task, multi-step planning, new lambda synthesis, or Tier A's own confidence estimate is low, *and* the content isn't privacy-tagged. Tier A packages the task context and calls the cloud client; Tier B returns a structured plan (function specs + UI patch intents, matching the parent doc's own description in §3.5); Tier A then executes that plan turn-by-turn, including any follow-up `lambda.register` / `policy.check` calls, rather than Tier B talking to MCP directly. This keeps exactly one component (Tier A, always-resident) as the actual MCP caller, which simplifies the Broker's provenance tagging (`policy-broker-spec.md` §8) — every capability request traces to the same session-loop identity regardless of which tier reasoned about it.

**No static threshold table** (per parent §7.4, deliberately left open there and deliberately *not* closed here): "low confidence" and "novel task" are properties Tier A's own prompt defines and can be tuned per-user via the skill library (§5), not a hardcoded number in the Rust harness. This is a direct instance of design goal §0.1 — if there were a compiled `if confidence < 0.7` anywhere, that would be task-specific logic leaking into the harness.

---

## 4. The privacy hard rule, structurally enforced

- Every wake context carries a `privacy_tag` computed by whatever produced the underlying event — the UI Runtime tags text/voice input that touched mic/camera capture or a `CAP_FS_READ`-scoped personal path; a lambda's `task-complete` event carries forward the tag of the data it processed. This tagging is a property of `local-model-spec.md`'s ingestion path and the Event Bus's payload shape (`event-bus-spec.md` §1), not something Tier A itself decides after the fact.
- The MCP Client component (not Tier A's reasoning) refuses to route a Tier B (cloud) call if the outbound context carries a `privacy_tag` — this is a check in the compiled harness, the one piece of "hardcoded logic" this spec explicitly carves out an exception for, because §0.5 treats this as a hard rule rather than a judgment call. Tier A cannot argue its way past this check; there is no prompt path that reaches the cloud client with tagged content, because the client call itself is gated below the reasoning layer.
- User-opted cloud escalation for privacy-tagged content (parent §6 mentions this as a possible future affordance) is out of scope for this version — it would require a Broker-mediated `CAP_CLOUD_ESCALATE` grant with its own `CONFIRM` policy, not a Tier A decision, and is deferred (§12).

---

## 5. Skill/prompt library

- Task intelligence is a versioned library of prompts + few-shot skills, loaded by the session loop at each wake based on `wake_reason.category` (mirroring the Event Bus taxonomy, `event-bus-spec.md` §1) — a `category: input` wake loads the general intent-classification skill; a `category: health` wake loads a much narrower "should this crash escalate to a UI notification or self-heal silently" skill.
- Skills are data, not code: they are read by the harness, sent as part of the model call's system context, and never executed. This is what keeps the "no task-specific branching in compiled code" property true even as the system's behavior grows more sophisticated over time — growth happens in the skill library, versioned and rollback-able the same way lambdas and policies are, not in a new Rust release.
- The skill library itself is Broker-scoped storage (`state.get`/`state.set` under a reserved `task.agent_skills.*` prefix, `state-store-spec.md` §5) — updating a skill is a capability-gated write, auditable like anything else.

---

## 6. Retirement mechanic

When Tier A or Tier B decides a capability should exist as a standing thing rather than be re-reasoned about every time, the plan it emits includes a `lambda.register(..., exposes_mcp=... )` or `handles_event=...` manifest field (`auil-asl-spec.md` §8, `event-bus-spec.md` §2). From that point forward, the Agent Core is not invoked again for that intent family — the routing tables in the MCP Bus and Event Bus point directly at the registered lambda. The Agent Core's aggregate long-run behavior is therefore a *shrinking* footprint over the system's lifetime for any given user's common tasks, which is the intended shape per the parent doc's opening line ("the agent decides what, never how") taken to its logical endpoint.

---

## 7. Systemd / kernel control surface

- Narrow, D-Bus-backed MCP surface: `systemd.status`, `.start`, `.stop`, `.restart`, `.enable`, `.disable`, `.logs` — a fixed, closed tool set, not general D-Bus passthrough.
- Every call still goes through `policy.check` (`policy-broker-spec.md` §4) like any other Agent Core action; the **protected-unit list** (`policy-broker-spec.md` §5) is what makes actions against load-bearing units resolve to `CONFIRM` regardless of policy configuration.
- When a `CONFIRM` comes back, the Agent Core's session loop blocks on `policy.confirm_result` and does **not** attempt to render its own waiting/confirmation UI — the Confirmation Surface Daemon (`policy-broker-spec.md` §9) owns that entirely. The Agent Core may render an *inert* "waiting for confirmation" status patch elsewhere in the UI tree (so the user isn't confused about why nothing's happening), but that patch carries no affirmative control of its own — it cannot be the thing the user clicks to approve.

---

## 8. Boot sequence (detail on the parent doc's boot order)

```
GRUB → kernel → initramfs → systemd
  → policy-broker        (must be up before anything it would gate)
  → lambda-server
  → state-store           (needed by lambda-server's own health reporting and by everything downstream)
  → event-bus
  → systemd-control        (the narrow D-Bus MCP surface agent-core will call into)
  → compositor
  → agent-core             (loads local model — Tier A — synchronously; blocks its own
                             readiness signal until the local model finishes warm-loading,
                             per the parent doc's boot table)
  → agent-greet             (oneshot unit; triggers the login greeting patch, parent §4 step 4)
```

Tier B (cloud client) requires no boot-time initialization beyond having network reachability checked lazily on first escalation attempt — there's no "warm cloud connection" concept, unlike the local model's mandatory warm-load.

---

## 9. MCP surface exposed by Agent Core

| Tool | Purpose |
|---|---|
| `agent.status()` | Current session loop state (idle / reasoning-local / reasoning-cloud / awaiting-confirm), for diagnostics and the Fallback Shell's "agent unavailable" indicator |
| `agent.interrupt()` | Cancel the in-flight plan for the current wake (used if a new, higher-priority wake arrives mid-turn) |
| `agent.local_only_mode(bool)` | The hard system-setting toggle the parent doc calls for in §6 — when set, the MCP Client's Tier B gate (§4) is unconditionally closed regardless of privacy tags, enforced at the same layer |

Note this is a small surface — most of what the Agent Core *does* shows up as outbound calls (`lambda.*`, `state.*`, `policy.*`, `ui.patch`) rather than inbound tools other components call on it, since it's the orchestrator, not a service being orchestrated.

---

## 10. Failure / fallback interaction

- If the local model fails to load at boot, `agent-greet` never fires and `agent.status()` reports unavailable; per parent §3.7, the UI Runtime falls back to rendering the last known-good State Tree read-only, entirely without querying the Agent Core.
- If the session loop crashes mid-turn after having already emitted some MCP calls (e.g. a `lambda.register` succeeded but the follow-up UI patch never got emitted), the Agent Core is capability-scoped and process-isolated like any other component (§1) — its crash is a `health` event on the Event Bus like any lambda's, and its restart is handled by the same protected-unit `CONFIRM` policy as the Broker or State Store (`policy-broker-spec.md` §5), not silently auto-restarted, since a bad restart loop here would flap the entire system's decision-making layer.

---

## 11. Security summary

| Threat | Mitigation |
|---|---|
| Agent Core is treated as privileged and bypasses the Broker | No such path exists; every action is an ordinary `policy.check`-gated MCP call, same as a lambda (§1) |
| Privacy-sensitive content reaches the cloud model | Tier B routing is gated below the reasoning layer by a compiled check on `privacy_tag`, not by Tier A's judgment (§4) |
| Compromised skill library redirects agent behavior | Skill writes are capability-gated State Store writes, auditable and rollback-able like any other state (§5) |
| Agent renders its own confirmation dialog to bypass human review | Structurally prevented — confirmation rendering is owned entirely by the Broker's Confirmation Surface Daemon, not the Agent Core (§7, `policy-broker-spec.md` §9) |
| Agent Core crash-loop destabilizes the whole system | Crash/restart is a protected-unit action requiring the same out-of-band confirmation as any other load-bearing component (§10) |

---

## 12. Open items before implementation

1. **Confidence/novelty signal format** — what exactly Tier A hands to its own prompt to make the escalate/don't-escalate call (§3) needs a concrete schema, even though the *threshold* is deliberately left as a tunable judgment call.
2. **User-opted cloud escalation for privacy-tagged content** — deferred in §4; needs its own `CAP_CLOUD_ESCALATE` + `CONFIRM` policy design once the Broker's grant model (`policy-broker-spec.md` §2) is implemented.
3. **Context window / gather() budget** — `gather(wake_reason)` in §2 needs bounds on how much state/history it pulls before calling either model tier, to keep local-model latency low for the common case.
4. **Cloud client failover/offline behavior** — parent §6 requires graceful degradation when the cloud is unreachable; this doc doesn't yet specify what Tier A does with a task it *would* have escalated when Tier B is unreachable (proceed locally with a caveat? queue and retry? surface a "needs internet" status to the user?).
5. **Multi-wake coalescing** — if the Event Bus coalesces rapid repeated wakes for the same category (`event-bus-spec.md` §6), does the Agent Core ever need to see the *count* of coalesced events, or is "at least one more happened" always sufficient context?

### Implementation Reference (auto-generated)

*No Python modules scanned under `agent-core` yet (design draft).*


---

# local-model/docs/spec.md
# L4Local Model Interface Specification

## Overview
The L4Local Model Interface provides a Tier A runtime with an always-on small model (3B-parameter quantized model like Phi-3). It integrates with the Policy Broker, Event Bus, and State Store for privacy tagging, health reporting, and embedding caching.

## Core Features
- **Tier A Runtime**: Always-on small model (e.g., Phi-3-mini-4k-instruct).
- **Privacy Tagging**: Stamp outputs with `privacy_tag` if input touched `CAP_MIC`/`CAP_CAMERA`/`CAP_FS_READ`.
- **Embedding Backend**: Power `lambda.search` semantic ranking.
- **Health Reporting**: Feed Event Bus (`category: health`).
- **MCP Interface**: Expose `localmodel.complete`, `localmodel.classify_intent`, `localmodel.embed`.

## Dependencies
- **Policy Broker**: Validate `CAP_IPC_CALL(targets=[localmodel])`.
- **Event Bus**: Publish `health` events.
- **State Store**: Cache embeddings for `lambda.search`.

## Implementation Details
- **Inference Engine**: `llama.cpp` (C++ backend with Python bindings).
- **Model**: Quantized 3B-parameter model (e.g., Phi-3).
- **Sandboxing**: Firecracker microVM or gVisor (stub for now).
- **Privacy Tagging**: Add `privacy_tag` to outputs if input touched sensitive capabilities.

## MCP Interface
- **Endpoints**:
  - `POST /mcp/localmodel.complete` → `{"text": "...", "privacy_tag": "..."}`
  - `POST /mcp/localmodel.classify_intent` → `{"intent": "media.play", "confidence": 0.95}`
  - `POST /mcp/localmodel.embed` → `{"embedding": [...], "privacy_tag": "..."}`
  - `GET /mcp/localmodel.health` → `{"status": "healthy", "load": 0.75}`
- **Protocol**: FastAPI + JSON.

## Example Usage
```python
from local_model.engine import LocalModelEngine
from local_model.models import CompletionRequest

engine = LocalModelEngine(model_path="/models/phi-3-q4.gguf")

# Complete text
response = engine.complete(CompletionRequest(
    prompt="Play a YouTube video of",
    max_tokens=50,
    privacy_tags=["CAP_MIC"]
))
print(response.text)  # Output: "rick astley never gonna give you up"
print(response.privacy_tag)  # Output: "CAP_MIC"

# Generate embeddings
embedding = engine.embed(EmbeddingRequest(text="Play YouTube"))
print(embedding.embedding)  # Output: [0.1, -0.3, ...]
```

## Project Structure
```
local-model/
├── pyproject.toml       # Poetry config + dependencies
├── local_model/
│   ├── __init__.py
│   ├── engine.py        # llama.cpp integration
│   ├── mcp_server.py    # FastAPI MCP interface
│   ├── models.py        # CompletionRequest, EmbeddingRequest
│   ├── privacy.py       # Privacy tagging logic
│   └── health.py        # Health reporting
├── docs/
│   └── spec.md          # Spec
└── tests/
    ├── test_engine.py
    └── test_mcp_server.py
```

### Implementation Reference (auto-generated)

Scanned `local-model`. Module / public-symbol inventory:

- **`local_model/engine.py`**
  - `class LocalModelEngine`
    - `complete()`
    - `embed()`
    - `classify_intent()`
- **`local_model/health.py`**
  - `def get_health_status()`
- **`local_model/mcp_server.py`**
  - `def complete()`
  - `def embed()`
  - `def classify_intent()`
  - `def health()`
- **`local_model/models.py`**
  - `class CompletionRequest(BaseModel)`
  - `class CompletionResponse(BaseModel)`
  - `class EmbeddingRequest(BaseModel)`
  - `class EmbeddingResponse(BaseModel)`
  - `class IntentRequest(BaseModel)`
  - `class IntentResponse(BaseModel)`
  - `class HealthResponse(BaseModel)`
- **`local_model/privacy.py`**
  - `def get_privacy_tag()`

**Tests discovered:** 8 `test_*` functions.


---

# UI Engine — Declarative Renderer, AUIL/ASL Parser & Patch Protocol

**Fills:** §3.6.2 of `docs/spec.md` (Declarative UI Runtime)  
**Related:** `state-store-spec.md` (UI State Tree persistence), `agent-core-spec.md` (ui.patch tool)  
**Version:** 0.1  
**Status:** Implemented

---

## Overview

The UI Engine is the L5 declarative renderer. It consumes the **UI State Tree** from the State Store and draws it through a pluggable `AbstractRenderer`.

## Modules

| Module | Responsibility |
|--------|----------------|
| `auil_parser.py` | Parse AUIL (Agent UI Layout) into `UINode` trees |
| `asl_parser.py` | Parse ASL state bindings |
| `patch_protocol.py` | Parse and apply patch operations (`~`, `+`, `-`, `!`, `@`) |
| `runtime.py` | `UIRuntime` — holds the live UI State Tree |
| `renderer.py` | `TreeRenderer` + `AbstractRenderer` interface |
| `models.py` | `UINode`, `UIStateTree`, `PatchOperation` |
| `components.py` | Primitive component definitions |
| `mcp_interface.py` | MCP tools: `ui.patch`, `ui.get`, `ui.bind` |
| `server.py` | HTTP + MCP server entry point |

## AUIL example

```
stack#root dir=v gap=m
  text(role=title) "Hello World"
  button#ok label=OK on:press=mcp:app.confirm
```

## Patch protocol

```
~footer(color=accent)
+footer/append: text(role=caption) "Copyright"
-old-banner
```

## Tests

```bash
cd ui-engine && python3 -m pytest test_engine.py
cd ui-engine-demo && python3 -m pytest test_demo.py
```

### Implementation Reference (auto-generated)

Scanned `ui-engine`. Module / public-symbol inventory:

- **`asl_parser.py`**
  - `class ASLParser`
    - `parse()`
    - `get_errors()`
  - `def parse_asl()`
- **`auil_parser.py`**
  - `class AUILParser`
    - `parse()`
    - `get_errors()`
  - `def parse_auil()`
- **`components.py`**
  - `class ComponentRegistry`
    - `register()`
    - `get()`
    - `resolve_mixins()`
    - `resolve_slots()`
    - `create_instance()`
    - `list_components()`
    - `validate_slots()`
- **`mcp_interface.py`**
  - `class MCPControlInterface`
    - `handle_tool_call()`
    - `get_available_tools()`
- **`models.py`**
  - `class PrimitiveTag(Enum)`
  - `class TextRole(Enum)`
  - `class MediaType(Enum)`
  - `class ChartType(Enum)`
  - `class LayoutDirection(Enum)`
  - `class PatchOp(Enum)`
  - `class EventType(Enum)`
  - `class ReferenceType(Enum)`
  - `class AdaptiveColor`
    - `resolve()`
  - `class DesignToken`
  - `class MotionCurve`
  - `class Scale`
  - `class StateTransition`
  - `class StyleMixin`
  - `class Reference`
    - `parse()`
  - `class Property`
    - `parse()`
  - `class UINode`
    - `path()`
    - `find_by_id()`
    - `to_dict()`
  - `class PatchOperation`
    - `parse()`
  - `class SlotDefinition`
  - `class ComponentDefinition`
  - `class UIStateTree`
    - `find_node()`
    - `apply_patch()`
    - `to_dict()`
- **`patch_protocol.py`**
  - `class PatchParser`
    - `parse()`
    - `parse_batch()`
    - `get_errors()`
  - `class PatchApplicator`
    - `apply()`
    - `get_stats()`
    - `reset_stats()`
  - `def parse_patches()`
- **`renderer.py`**
  - `class RenderCommand`
  - `class AbstractRenderer(ABC)`
    - `create_surface()`
    - `update_surface()`
    - `destroy_surface()`
    - `commit_batch()`
    - `flush()`
    - `get_surface_state()`
  - `class MockRenderer(AbstractRenderer)`
    - `create_surface()`
    - `update_surface()`
    - `destroy_surface()`
    - `commit_batch()`
    - `flush()`
    - `get_surface_state()`
    - `get_commands()`
    - `clear_commands()`
    - `get_surfaces()`
  - `class TreeRenderer`
    - `render()`
    - `update()`
- **`runtime.py`**
  - `class UIRuntime`
    - `set_render_callback()`
    - `load_auil()`
    - `load_asl()`
    - `load_styles()`
    - `apply_patch()`
    - `apply_patches()`
    - `find_node()`
    - `update_node()`
    - `get_node_properties()`
    - `set_state()`
    - `clear_state()`
    - `get_active_states()`
    - `resolve_mcp_intent()`
    - `get_tree()`
    - `get_stats()`
- **`server.py`**
  - `class UIEngine`
    - `render()`
    - `patch()`
    - `handle_mcp_tool()`
    - `get_tools()`
    - `get_renderer()`
    - `set_renderer()`
    - `get_stats()`
  - `def create_engine()`

**Tests discovered:** 10 `test_*` functions.


---

# UI Engine Demo — Pure-Wayland Vertical Slice

**Status:** Implemented. A runnable end-to-end vertical that exercises the full UI
Engine stack (AUIL parse → patch → render → input) without a real GPU.

`ui-engine-demo/` is the reference application that proves the UI Engine design is not
just a spec. Because `wlroots` is a C library (not pip-installable) and this repository
is a logic/architecture project, the demo ships a **terminal-based renderer** that
implements the UI Engine's `AbstractRenderer` interface. The same runtime code that
drives the terminal demo is what a future `wlroots` compositor backend would drive —
only the renderer implementation changes.

---

## 1. Files

| File | Role |
|---|---|
| `demo.auil` | AUIL layout: a root `stack`, a title `text`, an input `field`, a submit `button`, and an output `text`. |
| `wayland_renderer.py` | `WaylandRenderer` — a terminal implementation of `AbstractRenderer` (`create_surface`, `update_surface`, `destroy_surface`, `commit_batch`, `flush`, `get_surface_state`). |
| `demo.py` | App entry point. Loads `demo.auil`, drives the `Runtime`, and runs a raw-mode keyboard input loop (via `tty`/`termios`) that feeds events into the patch pipeline. |
| `test_demo.py` | 20 tests covering parser, renderer, runtime, and the patch→render→output flow. |

---

## 2. The flow

1. `demo.auil` is parsed by `ui_engine.auil_parser` into a node tree.
2. The tree is handed to `ui_engine.runtime.Runtime`, which holds the live UI State Tree.
3. User keystrokes arrive through the input loop and are translated into **patches**
   (`ui_engine.patch_protocol`): `~id(props)` to update text, `+path node` to insert,
   `-id` to remove.
4. Patches are applied to the State Tree and pushed to the `WaylandRenderer`, which
   repaints the terminal.
5. A submit action reads the input field and writes the result into the output label —
   demonstrating input → state → output with no LLM in the real-time path.

---

## 3. Why a terminal renderer

The parent architecture commits to *real-time paths never touching inference*. The
terminal renderer honors that: it is fully deterministic and depends only on the
`AbstractRenderer` contract. Swapping in a `wlroots` surface later requires no changes
to the parser, patch protocol, or runtime — only a new `AbstractRenderer`
implementation. This is the practical expression of the UI Engine's "pluggable
renderer" design.

---

## 4. Known integration seam

The demo's patch tests exercise `ui_engine.patch_protocol` directly. That module is the
shared contract between the Agent Core's emitted patches and the UI Runtime's
application step, so any bug there surfaces first in `test_demo.py`. It is the natural
place to extend when wiring the Agent Core's `ui.patch` tool to a live surface.

---

## 5. Running it

```bash
cd ui-engine-demo
uv run python demo.py          # interactive terminal app
uv run pytest test_demo.py -q   # test suite
```

[TOC]

### Implementation Reference (auto-generated)

Scanned `ui-engine-demo`. Module / public-symbol inventory:

- **`demo.py`**
  - `class App`
    - `start()`
    - `run()`
  - `def main()`
- **`wayland_renderer.py`**
  - `class TerminalSurface`
  - `class WaylandRenderer`
    - `set_event_handler()`
    - `create_surface()`
    - `update_surface()`
    - `destroy_surface()`
    - `commit_batch()`
    - `flush()`
    - `get_surface_state()`
    - `get_surfaces()`
    - `clear()`
    - `handle_key()`

**Tests discovered:** 20 `test_*` functions.


---

# MCP Bus — Message Fabric, Intent Registry & Handler Resolution

**Fills:** §3.4 of `agent-native-os-architecture.md` (MCP Bus) — the component `auil-asl-spec.md` §8 and `event-bus-spec.md` §2 both assume exists but never specify
**Related:** `auil-asl-spec.md` §8 (three-way handler classification this doc implements the routing table for), `lambda-server-spec.md` §7 (`exposes_mcp` registration), `event-bus-spec.md` §2 (`handles_event` registration — a parallel registry this doc's mechanism generalizes), `policy-broker-spec.md` §11 (registration validation)
**Version:** 0.1  
**Status:** Partially implemented — see `mcp-bus/src/` (dynamic registry, `bus.resolve`, `_bus.register`)

---

## 0. Design goals

1. **The bus is a router, not a component.** Every other spec in this project describes something with meaningful internal state and behavior (a registry of functions, a store of values, a queue of events). The MCP Bus deliberately has almost none of that — its entire job is "given a method name, find the process that should handle it, forward the call, forward the response." Keeping it this thin is what makes "every layer talks to every layer only through MCP" (parent §3.4) actually cheap enough to be a universal rule rather than an aspiration.
2. **One registry, many owners.** `auil-asl-spec.md` §8 talks about "the L3 Bus's intent registry," `lambda-server-spec.md` §7 registers `exposes_mcp` entries into it, `event-bus-spec.md` §2 registers `handles_event` entries into a routing table described as "the same mechanism." This spec makes that literal: there is one registry, one resolution algorithm, and the "AUIL intent" and "event pattern" cases are the same code path with different key namespaces, not two systems that happen to rhyme.
3. **Resolution is O(1) lookup, not negotiation.** A method call's route is decided by a registry lookup at call time, not by asking every possible handler "can you handle this?" — this is what keeps steady-state latency flat regardless of how many lambdas exist in the system (parent §3.4's stated benefit: "one protocol, one audit format, one place to enforce policy" only holds if resolution itself is cheap).
4. **The Bus enforces nothing beyond routing.** Capability checks are the Broker's job (`policy-broker-spec.md`), not this component's — the Bus resolves *who* would handle a call and forwards it; whether that call is *allowed* was already decided when the handler was registered (registration itself went through `policy.check`) or, for the fallthrough-to-agent case, is decided per-call by the Broker downstream of the Bus's routing decision.

---

## 1. Component map

```
┌──────────────────────────────────────────────────────────────────┐
│  MCP Bus (L3)                                                     │
│                                                                    │
│  ┌────────────────────────┐   ┌──────────────────────────────┐   │
│  │ Intent Registry          │   │ Call Router                   │   │
│  │ (method namespace →      │──►│ (resolve → forward → return   │   │
│  │  handler identity, one   │   │  response; no call-content    │   │
│  │  entry per namespace,    │   │  inspection beyond routing     │   │
│  │  versioned like a lambda)│   │  key extraction)               │   │
│  └────────────┬─────────────┘   └───────────────┬───────────────┘   │
│               │                                  │                   │
│  ┌────────────▼──────────────────────────────────▼───────────────┐  │
│  │        Connection multiplexer (one socket per component,        │  │
│  │        MCP framing in/out, no business logic)                   │  │
│  └──────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

Every component in the OS — Agent Core, Lambda Server, State Store, Event Bus, UI Runtime, Policy Broker itself — holds exactly one connection to the Bus. The Bus never initiates a call on its own; it only ever routes calls it received.

---

## 2. The registry — one table, several key namespaces

```
namespace: mcp-intent     — e.g. "player.toggle", "calc.add"     (registered via exposes_mcp, lambda-server-spec.md §7)
namespace: event-handler  — e.g. "task-complete.download"         (registered via handles_event, event-bus-spec.md §2)
namespace: system-op      — e.g. "systemd.restart", "power.set_profile"  (fixed, shipped with the OS image, not agent-registerable)
namespace: state-op       — e.g. "state.get", "state.watch"        (fixed, always resolves to the State Store)
```

Each registry entry is: `{namespace, key, handler_identity, registered_at, registered_by, manifest_ref}`. `system-op` and `state-op` entries are pre-populated at boot from the OS image and are **not writable** via any MCP call — only `mcp-intent` and `event-handler` entries are ever registered at runtime, and only as a side effect of a Broker-validated `lambda.register` or event-subscription call (the Bus itself exposes no direct "register a route" tool to arbitrary callers — see §5).

**Resolution algorithm**, given an inbound call `method`:
1. Extract `namespace` from the method's fixed prefix convention (`state.*` → `state-op`, `systemd.*`/`power.*`/etc. → `system-op`, everything else → check `mcp-intent` then `event-handler` by exact key, in that order since a UI-authored `on:press=mcp:` call and an event pattern share the dotted-namespace shape but never the same literal key in practice).
2. If a matching entry exists → forward to `handler_identity` (a lambda, the State Store, the System Daemon).
3. If no entry exists in `mcp-intent`/`event-handler` → fall through to the Agent Core (this is the "first press" case in `auil-asl-spec.md` §8 step 2, and the "no registered handler" case in `event-bus-spec.md` §2's table).
4. `system-op`/`state-op` calls with no entry are a configuration error, not a fallthrough case — they always resolve, by construction, since they're fixed at boot.

This is exactly the three-way table `auil-asl-spec.md` §8 describes (Agent Core / lambda-hosted MCP server / Broker-System-Daemon), generalized: the Bus doesn't know or care *which* of those three a resolved handler is — it's just an identity to forward to. The "is this deterministic or does it involve inference" distinction lives in what that identity happens to be, not in the Bus's logic.

---

## 3. Registration lifecycle

- Registration is never a direct Bus call. It's a side effect the Bus observes: when the Broker approves a `lambda.register(..., exposes_mcp=X)` or an `event.subscribe`/manifest declaring `handles_event=Y` (both already validated per `policy-broker-spec.md` §11), the approving component (Lambda Server or Event Bus respectively) tells the Bus "add this route" via an internal, non-agent-reachable registration call.
- This keeps the Bus from needing its own opinion about capability validity — by the time it sees a registration request, the decision was already made by the Broker. The Bus's only remaining job is rejecting a registration that collides with an existing key in the same namespace (exclusivity — mirrors `event-bus-spec.md` §7's "two lambdas race to claim the same `handles_event` pattern" concern, generalized to `mcp-intent` too).
- Deregistration happens automatically when the owning lambda is deprecated/rolled back (`lambda-server-spec.md` §7, `lambda.deprecate`) or when a `handles_event` manifest is superseded — the Bus subscribes to those lifecycle events rather than requiring an explicit "unregister" call from anyone.

---

## 4. Call forwarding

- The Bus does not buffer, retry, or transform payloads — it is a dumb pipe once resolution is done, on purpose (design goal §0.1). Retries, timeouts, and backpressure are the calling component's problem (the Agent Core's session loop already has to handle a stalled `lambda.invoke`; adding a second retry layer inside the Bus would just create two places that disagree about what "timed out" means).
- Streaming calls (e.g. `state.watch`'s long-lived subscription, `lambda-server-spec.md` §4.2's fast-path lease negotiation) are supported as long-lived forwarded connections, not specially modeled — the Bus keeps the multiplexed connection open and continues forwarding frames both directions until either side closes it.
- **Fast-path leases bypass the Bus entirely** by design, per `lambda-server-spec.md` §4.2 — a leased socket is process-to-process, established once via the Bus/Router and then used directly. The Bus's resolution cost is paid once per lease, not once per call within that lease's lifetime, which is the mechanism that keeps hot-loop IPC (a tight `x` → `y` call cycle) off the Bus's steady-state load entirely.

---

## 5. MCP surface

| Tool | Purpose |
|---|---|
| `bus.resolve(method)` | Introspection: what would this method currently route to, and via which namespace? (Analogous to `event-bus-spec.md`'s `bus.explain_routing`, generalized to all four namespaces.) |
| `bus.list_routes(namespace?)` | Enumerate current registry entries — audit/debugging, read-only. |
| — | There is deliberately no `bus.register` tool exposed to general callers; registration only happens as the internal side effect described in §3. This is the one place this spec departs from "every capability is an explicit MCP call" — registration is *observed*, not *requested*, specifically so the Bus can't become a second place (alongside the Broker) where a registration decision could be made. |

---

## 6. Interaction with the Policy Broker's audit log

Per `policy-broker-spec.md` §7, intent-registry registrations are logged **per-registration**, not per-call. This spec is where that boundary is physically drawn: the Bus's high-frequency call forwarding (§4) never touches the Broker or the audit log at all — only the registration event (§3), which originates from the Broker's own approval in the first place, produces an audit entry. The Bus is intentionally invisible to the audit trail on the hot path, the same way motion events are invisible to it by construction (`auil-asl-spec.md` §3.4).

---

## 7. Security summary

| Threat | Mitigation |
|---|---|
| A lambda registers itself for a method namespace it wasn't approved for | Registration is never a direct Bus-reachable call; it's an internal side effect of an already-Broker-validated `lambda.register`/`event.subscribe` (§3) |
| Two components race to claim the same intent/event key | Bus enforces per-namespace key exclusivity at registration time, rejecting the second claimant outright (§3) |
| A compromised component floods the Bus with resolution requests to enumerate the whole registry | `bus.list_routes`/`bus.resolve` are read-only and rate-limited like any MCP surface (`policy-broker-spec.md` §6); they reveal routing targets, not payloads, so enumeration alone isn't a data-exfiltration path |
| Stale route after a lambda crashes/rolls back | Deregistration is driven by the same lifecycle events the Lambda Server already emits (`lambda-server-spec.md` §9's rollback mechanics), not a separate heartbeat the Bus has to maintain itself |

---

## 8. Open items

1. ~~**Wire protocol between Bus and components**~~ — **Decided:** newline-delimited JSON over Unix sockets (one UTF-8 object + `\n` per message). Every boot daemon uses this framing. Length-prefixed / MessagePack remains a future optimization, not the current contract.
2. **Namespace prefix collision rules** — the "extract namespace from method prefix" resolution (§2) needs a formal grammar so `mcp-intent` keys can never accidentally shadow a `system-op`/`state-op` prefix.
3. ~~**Registry persistence across Bus restarts**~~ — Bus persists dynamic routes to `perm.mcp_routes.*` and reloads them on start (`reload_routes_from_state`).
4. **Multi-instance / sharding** — out of scope for a single-user machine today, but worth flagging that this design assumes exactly one Bus instance; nothing here has been designed for horizontal scaling.

---

# System Daemon — Raw I/O Ownership & the Narrow Kernel-Parameter MCP Surface

**Fills:** §3.1 of `agent-native-os-architecture.md` (the "small System Daemon" described at L0)
**Related:** `policy-broker-spec.md` §5 (schema-validated kernel/systemd ops this daemon executes), `policy-broker-spec.md` §9 (physically-originated input path this daemon owns, reused for confirmation-surface input provenance), `mcp-bus-spec.md` §2 (`system-op` namespace, pre-populated from this daemon's fixed tool set)
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation

---

## 0. Design goals

1. **As little code as possible, running as early as possible.** This is the one component that has to exist before the Bus, the Broker, or anything MCP-shaped is up, because raw input has to flow to *something* the instant the kernel hands it over. Every design choice here favors "small enough to audit by reading it once" over "flexible."
2. **Mechanism only, no policy.** The Daemon never decides *whether* a kernel-parameter change should happen — that's the Broker's job (`policy-broker-spec.md` §5). It only executes a fixed, pre-approved operation once told to, and only ever offers operations that were already whitelisted at the OS-image level, not ones it could be reconfigured into at runtime.
3. **The real-time input path never gets slower because of this OS's other ambitions.** Keyboard/mouse/audio latency here has to match (or beat) a conventional Linux desktop's input path — this daemon is not where "agent-native" trades away basic responsiveness.
4. **The one place input provenance is physically true.** `policy-broker-spec.md` §9's confirmation-surface design leans on there being a hardware-to-software path that nothing above L0 can synthesize. This spec is where that guarantee actually originates — everything above just trusts it.

---

## 1. Component overview

- A small, non-LLM daemon written in Rust or C, started effectively at PID-1-adjacent priority, before the compositor, before the Lambda Server, before anything MCP-facing (see parent boot order and `agent-core-spec.md` §8's fuller boot sequence — this daemon is up before `policy-broker` even starts).
- Owns raw device access: `evdev`/`libinput` for keyboard/mouse/touch, ALSA/PipeWire for audio device enumeration and routing, DRM/KMS hotplug notifications for monitor connect/disconnect, and the network interface list.
- Two responsibilities, kept structurally separate inside the daemon so a change to one can't accidentally affect the other:
  1. **Input forwarding** (§2) — zero MCP involvement, pure kernel-to-compositor plumbing.
  2. **Kernel-parameter MCP surface** (§3) — the only part of this daemon that speaks MCP at all.

---

## 2. Input forwarding — the real-time path

- Raw input events (keydown/up, pointer motion, button state, audio buffer callbacks) are forwarded directly to the Wayland Compositor (`compositor-spec.md`) over a dedicated, non-MCP local channel — this is Design Commitment #2 from the parent doc made concrete: this path has no MCP framing, no Broker check, no possibility of an agent or lambda intercepting it, because it was never routed through anything that could.
- **This is also the physically-originated input path** `policy-broker-spec.md` §9 requires for confirmation-surface approval: an input event that arrived via this daemon's forwarding path carries a provenance marker (a kernel-timestamp + device-id pair, not a claim any software component asserts about itself) that the Confirmation Surface Daemon checks for. No software component above this layer — including the Agent Core — has a code path that produces an event carrying this marker without it actually having come from a physical device, because the marker is stamped by this daemon reading directly from the kernel's input subsystem, not by anything downstream re-asserting it.
- Audio/video buffer plumbing (playback, capture) similarly never touches this daemon's MCP surface — the Lambda Server's media-handling lambdas (parent §3.6.2's video player example) get GPU/audio access via the mediated device path described in the parent doc (§3.2.1), not via a call to this daemon; this daemon's involvement in media is limited to owning the hotplug/device-enumeration events that tell the rest of the system what hardware exists.

---

## 3. Kernel-parameter MCP surface

- A **fixed, versioned, closed set** of operations, matching `policy-broker-spec.md` §5's schema-validation requirement exactly — this daemon does not accept free-form parameter names or values under any circumstance, including from the Broker itself. If an operation isn't in the compiled-in table below, there is no code path that executes it, full stop; expanding this set requires an OS image update, not a runtime registration.

```
power.get_profile / power.set_profile(profile: balanced|performance|powersave)
display.get_modes(output) / display.set_mode(output, mode)
net.list_interfaces() / net.set_interface_state(iface, up|down)
net.get_wifi_status() / net.connect_wifi(ssid, credential_ref)   — credential_ref points into a
                                                                    Broker-gated secret store, never
                                                                    a raw password in the call
audio.list_devices() / audio.set_default(device_id, role: output|input)
```

- Every call arrives already carrying a Broker-issued grant token (`policy-broker-spec.md` §4) — this daemon verifies the token's signature and scope match the requested operation before executing, but does **not** itself re-implement policy logic; a token that's structurally valid but was issued for a different operation is rejected here as a second, cheap check, not as this daemon's own judgment call about whether the action is a good idea.
- Results and current values are read-only queries with no side effects (`power.get_profile`, `display.get_modes`, etc.) and require no grant token at all — read access to "what state is the hardware in" is not gated, only mutation is, mirroring the general shape of `state-store-spec.md`'s read/write asymmetry.

---

## 4. Boot behavior

- Starts immediately after the kernel/initramfs hand-off, before `policy-broker` in the boot order (`agent-core-spec.md` §8) — this is a deliberate exception to "everything above L2 talks through the Broker," because at this point in boot there is no Broker yet to talk through. The daemon's kernel-op surface (§3) simply refuses all mutating calls until it observes the Broker come up and establish its own connection (a one-time handshake, not a per-call check at this stage) — before that handshake, only the read-only queries and input-forwarding path are live.
- This means the very earliest boot UI (parent §4 step 3 — compositor renders whatever the State Tree holds) can already display real display/audio hardware state without waiting for the Broker or Agent Core, which is what makes the Fallback Shell (`fallback-shell-spec.md`) able to show real system status even in the earliest, most degraded boot state.

---

## 5. Security summary

| Threat | Mitigation |
|---|---|
| Free-form kernel writes requested by a compromised agent | No such call shape exists — only the fixed, compiled-in operation table is executable (§3), matching `policy-broker-spec.md` §5's schema-validation stance from the other side |
| Forged input events used to fake human confirmation | Provenance marker is stamped from direct kernel-input reads inside this daemon; no software path above it can produce the marker without a real physical event (§2) |
| Grant-token replay against a different operation | Token scope is checked against the actual requested operation at execution time, not just verified as "signed by the Broker" (§3) |
| Daemon itself compromised (it runs with real device access) | Minimized code surface and early, narrow startup are the primary mitigation — this spec deliberately doesn't add features to this component, since every added feature is added attack surface at the one layer that can't be sandboxed the way L1 lambdas are |

---

## 6. Open items before implementation

1. **Credential store for `net.connect_wifi`** — `credential_ref` needs a concrete secret-storage design; this spec assumes one exists but doesn't define it.
2. **Hotplug event schema** — the shape of monitor/audio-device hotplug notifications forwarded toward the compositor and Event Bus (`event-bus-spec.md` §1, `category: external`) needs to be nailed down precisely.
3. **Handshake protocol with the Policy Broker** (§4) — "observes the Broker come up" needs an actual mechanism (a well-known socket path the Broker connects to on startup, most likely), not just a description.
4. **Provenance marker format** — the exact structure of the kernel-timestamp + device-id pair (§2), and how the Confirmation Surface Daemon verifies it wasn't replayed from a captured earlier event.

---

# Fallback / Degraded-Mode Shell — Zero-Inference Recovery UI

**Fills:** §3.7 of `agent-native-os-architecture.md` (Fallback / Degraded-Mode Shell)
**Related:** `state-store-spec.md` §3 (last-known-good revision, what this shell reads), `agent-core-spec.md` §10 (what triggers handoff to this shell), `local-model-spec.md` §6 (the specific failure this shell exists to survive), `system-daemon-spec.md` §4 (why real hardware status is available even this early), `policy-broker-spec.md` (still enforced even here — this shell is not a bypass)
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation

---

## 0. Design goals

1. **Zero inference, not "less" inference.** This is not a smaller/dumber agent. There is no model in this component's path at all, by construction — the entire point is that it works when every model in the system (local *and* cloud) is unavailable, so it cannot itself depend on either.
2. **Boots before, and outlives, everything above L2.** Per parent §3.7, this has to be usable "before local model loads" — meaning it doesn't wait on the Agent Core, the Lambda Server being warm, or even the Broker being fully initialized (though the Broker's own boot happens very early per `agent-core-spec.md` §8, well before the Agent Core). This shell's dependency graph is deliberately the shortest one in the entire OS.
3. **Read the truth, don't reconstruct it.** The State Store already defines what "last known-good" means precisely, via its monotonic revision counter (`state-store-spec.md` §3). This shell trusts that definition completely rather than inventing its own notion of "good enough to show."
4. **Still a citizen of the security model.** Being the recovery path doesn't mean being outside the Broker's authority — a "restart agent" or "safe-mode terminal" action from this shell still goes through `policy.check` like anything else. Degraded mode is a UI/inference condition, not a permissions bypass.

---

## 1. Component overview

- A small, statically-linked, dependency-minimal binary (same implementation posture as the System Daemon — auditable by reading it once) that can render a fixed, non-declarative UI directly via the compositor's client protocol, with no dependency on the AUIL/ASL parser or the UI Runtime process at all.
- Two operating modes:
  1. **Frozen last-good view** — renders a static snapshot of the last committed `ui.<tree>` revision from the State Store, read-only, with a persistent "agent unavailable" banner overlaid. This is not a live AUIL render (that would require the UI Runtime, which may itself be the thing that's down); it's closer to a screenshot reconstructed from the tree's text content and layout metadata, rendered by this shell's own minimal, fixed renderer.
  2. **Recovery console** — a small fixed set of deterministic actions (§3), available regardless of whether a frozen view could be produced (e.g. very early boot, before any tree has ever been committed).

---

## 2. Trigger conditions (when this shell takes over)

Per `agent-core-spec.md` §10, the primary trigger is `localmodel.health()` reporting not-ready (`local-model-spec.md` §6), observed via the Event Bus's `health` category — but this shell doesn't require the Event Bus to be functioning either, since that itself might be part of what's down. Concretely, this shell activates on any of:

- Boot-time: before `agent-core` unit's readiness signal fires (parent boot order, `agent-core-spec.md` §8) — covers cold boot before the local model finishes warm-loading.
- Runtime: the compositor observes the UI Runtime's client connection drop without a clean shutdown, or the Agent Core's `agent.status()` becomes unreachable for longer than a short grace period.
- Explicit: a user-invoked "safe mode" action (e.g. a fixed key combination captured by the System Daemon's real-time input path, `system-daemon-spec.md` §2 — deliberately not an AUIL-authored button, so it works even if the UI Runtime is the thing that's broken).
- Resource exhaustion signals from the Process Supervisor (`lambda-server-spec.md` §3) affecting the Agent Core or local model specifically.

This shell does not itself decide "is the agent degraded" via any heuristic of its own — it only reacts to explicit, already-computed signals (health reports, connection state, a hardware key) from components whose job it already is to know that. Inventing a second opinion here would just create a second thing that could be wrong.

---

## 3. Recovery console — fixed action set

```
view_status       — hardware state via system-daemon read-only queries (power, display, network),
                     available with zero MCP dependency beyond the System Daemon itself
view_logs         — tail the Policy Broker's audit log (policy-broker-spec.md §7) and
                     recent Event Bus health events, read-only
restart_agent     — systemd.restart against the agent-core unit; goes through the normal
                     policy.check path (policy-broker-spec.md §4/§5) like any systemd action —
                     if agent-core is on the protected-unit list, this still requires the
                     out-of-band confirmation (policy-broker-spec.md §9), even from this shell
connect_network   — thin wrapper over system-daemon's net.* operations (system-daemon-spec.md §3),
                     needed so a user can get online to, e.g., pull an OS update that fixes
                     whatever caused the degraded state
safe_terminal     — a plain shell (bash/sh), gated behind the same protected-action confirmation
                     as any other sensitive systemd/kernel action, not a free pass just because
                     the agent is down
```

Every action in this list is a fixed, hand-authored binding to an existing MCP tool already defined in another spec — this shell introduces no new capabilities of its own, only a UI path to invoke ones that already exist, specifically so it doesn't become a second, less-audited way to do sensitive things.

---

## 4. Rendering the frozen view without the UI Runtime

- This shell's renderer understands a **deliberately tiny subset** of what an AUIL tree can express: `text` content and `stack`/`grid` layout, enough to lay out roughly what was on screen, explicitly *not* attempting to reproduce ASL styling, motion, or `$lambda:`-bound live media (a frozen `media` node renders as a placeholder with its last-known label, not an attempt to resume playback).
- This is read directly from the State Store's `ui.<tree>` namespace (`state-store-spec.md` §1, §8) via `state.get` — no MCP intent invocation, no lambda calls, since a lambda might itself be part of what's unavailable.
- If the State Store itself is unavailable (a strictly worse failure than "just the model is down"), this shell has no frozen view to show and falls straight to the recovery console (§3) with an explicit "no cached UI state available" message rather than a blank screen — a blank screen with no explanation is exactly the failure mode this component exists to prevent.

---

## 5. Relationship to the Policy Broker

- This shell is not a privileged bypass of the Broker. Every mutating action in §3 is an ordinary `policy.check`-gated call, identical in shape to a call the Agent Core would make. The only difference is *who* is initiating it (a human, directly, through a fixed UI) rather than an LLM's plan — the Broker's decision model (`policy-broker-spec.md` §3) doesn't need to know or care about that distinction, since its job is evaluating the request, not the requester's nature.
- One consequence worth being explicit about: if the Policy Broker *itself* is down, this shell's mutating actions (§3) simply cannot proceed, by the same logic that nothing else in the OS can act without the Broker. `view_status` and `view_logs`' read-only paths still work, since they route around the Broker to begin with (read-only System Daemon queries, `system-daemon-spec.md` §3). This is treated as correct behavior, not a gap — a Broker outage is a more severe failure than a model outage, and this shell doesn't try to paper over it with an exception.

---

## 6. Security summary

| Threat | Mitigation |
|---|---|
| Degraded mode used as a way to skip Broker confirmation for sensitive actions | Every mutating recovery action still goes through the identical `policy.check`/`CONFIRM` path as normal operation (§5) |
| Attacker triggers a fake "agent unavailable" state to get the user into a less-audited UI | This shell's action set (§3) is exactly as audited as normal operation — there's no reduced-scrutiny mode to gain access to, so triggering degraded mode doesn't buy an attacker anything beyond what's already possible |
| Frozen view misrepresents current system state as if it were live | Persistent, unmissable "agent unavailable" banner is mandatory on the frozen view (§1); the renderer explicitly does not attempt to resume live bindings like media playback, which could otherwise mislead a user into thinking the system is functioning normally |
| Recovery console itself has a bug that's exploitable | Same minimal-code-surface posture as the System Daemon (§0.1) — this is the second component in the OS, alongside the System Daemon, held to "small enough to audit by reading it once" rather than "featureful" |

---

## 7. Open items before implementation

1. **Grace period tuning** for the "Agent Core unreachable" trigger (§2) — too short causes flapping into degraded mode on ordinary slow responses; too long delays a real recovery signal.
2. **Frozen-view fidelity** — how much of `ui.<tree>`'s layout metadata is worth preserving for this shell's tiny renderer versus just falling back to a plain list of text content; needs actual user testing once the UI Runtime's real tree shapes exist.
3. **Update mechanism access from safe mode** — `connect_network` (§3) implies a path to pull an OS update while degraded, but this doc doesn't specify the update mechanism itself (out of scope, per parent §7.6 — "update/rollback mechanics for the OS components themselves" is its own still-open item).
4. **Multi-output behavior** — if the frozen view needs to render across multiple displays, this shell's tiny layout subset (§4) may need explicit multi-output handling it doesn't currently address.

---

# Wayland Compositor — Integration Spec (wlroots-based, minimal custom surface)

**Fills:** §3.6.1 of `agent-native-os-architecture.md` (Wayland Compositor)
**Related:** `system-daemon-spec.md` §2 (input forwarding this compositor receives), `ui-engine`'s existing runtime (the primary client this compositor serves), `policy-broker-spec.md` §9 (the one non-standard protocol extension this compositor must add)
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation — intentionally thin; most of this component is "use existing software correctly," not novel design

---

## 0. Design goals

1. **Don't reinvent a compositor.** Nothing about the agent-native premise requires new compositing, damage-tracking, or frame-scheduling logic — wlroots already solves this well, and inventing a replacement would be pure risk with no corresponding benefit to the OS's actual thesis. This spec's job is to say precisely *which* wlroots-based behavior this OS depends on and *what's* different, not to redesign compositing.
2. **XWayland is the escape hatch, not the target.** Parent §3.6.1 is explicit that conventional Wayland/X11 clients can still run for software not worth reimplementing as a lambda-backed component (the CAD-tool example). This spec treats that as a compatibility feature to preserve, not extend — no work here goes toward making legacy apps feel "native" to the agent, since they're explicitly outside its model.
3. **Exactly one protocol addition beyond stock wlroots: the confirmation-surface role.** This is the sole piece of genuinely custom compositor work this OS needs, and it's small and well-precedented (session-lock protocols already establish the "reserved surface role only one specific client may bind" pattern).
4. **The compositor has no opinion about AUIL/ASL.** It renders whatever the UI Runtime (`ui-engine`) hands it as ordinary Wayland surfaces/buffers — AUIL patch semantics, ASL tokens, and mixin resolution are entirely the UI Runtime's problem (`auil-asl-spec.md`), not something the compositor parses or understands. To the compositor, the UI Runtime is just a normal (if privileged) Wayland client.

---

## 1. Base: stock wlroots behavior, unmodified

- Standard compositing, damage tracking, frame scheduling, multi-output handling, DRM/KMS backend — no changes from upstream wlroots behavior.
- Standard `xdg-shell` for the UI Runtime's own surfaces, and standard `XWayland` support for legacy/escape-hatch clients (parent §3.6.1's CAD-tool case).
- Input event delivery from the System Daemon (`system-daemon-spec.md` §2) is accepted over that daemon's dedicated local channel, translated into the same internal event representation wlroots would use for a normal libinput-sourced event — from the compositor's internal perspective, input "comes from libinput" in the conventional sense; the System Daemon's separate ownership of the raw device (parent §3.1) is a system-architecture distinction that doesn't require the compositor's input-handling code to look different from a stock build.

---

## 2. Client roles

| Client | Role | Privilege |
|---|---|---|
| **UI Runtime** (`ui-engine`) | Renders the AUIL tree; primary/default client | Ordinary `xdg-shell` surfaces, but treated as the always-present "desktop" client (analogous to a shell/panel process in a conventional compositor setup) |
| **Legacy Wayland/X11 apps** | Escape hatch (parent §3.6.1) | Ordinary surfaces, composited alongside/within whatever the UI Runtime's tree currently allocates space for them — the UI Runtime is responsible for giving a legacy app's surface a place in the AUIL tree (e.g. via a `media`-like "external surface" primitive), not the compositor deciding layout on its own |
| **Confirmation Surface Daemon** (`policy-broker-spec.md` §9) | The one non-standard role (§3) | Reserved protocol role; no other client, including the UI Runtime, may bind it |

---

## 3. The confirmation-surface protocol extension

- A new Wayland protocol extension, `confirmation-surface-v1`, modeled directly on the existing `ext-session-lock-v1` pattern: a client requests the role, the compositor grants it to **at most one client for the lifetime of the compositor process**, identified by a fixed, compile-time-known socket/credential the Broker's Confirmation Surface Daemon connects with at its own startup (mirrors how session-lock implementations typically restrict the role to a specific, trusted binary).
- While a `confirmation-surface-v1` surface is mapped, the compositor:
  - Renders it above all other surfaces, full attention (analogous to a lock-screen surface taking exclusive focus).
  - Routes all input exclusively to it — no input event reaches the UI Runtime or any other client while a confirmation surface is active, which is what prevents a race where a UI-Runtime-rendered element could visually overlap or intercept a click meant for the confirmation surface.
  - Refuses any *other* client's request to bind the same role while one is already granted, and refuses the request outright if the requesting client's credential doesn't match the one fixed identity established at compositor startup.
- This is the entirety of the compositor's custom work. It does not need to understand *why* the surface exists, what a policy decision is, or anything about the Broker's rule language (`policy-broker-spec.md` §2) — it only needs to enforce "this role, this one client, exclusive input and top-most rendering while mapped."

---

## 4. Vibrancy / backdrop-blur handling

- `auil-asl-spec.md` §3.2 assigns `vibrancy=` token resolution to "compositor-level backdrop blur," not something the UI Runtime computes itself. Concretely: the UI Runtime's surfaces declare a blur region + intensity via a standard-shaped compositor protocol (e.g. an implementation of `wp_fractional_scale`-adjacent or a custom minimal blur-region protocol if no suitable stock one exists), and the compositor performs the actual blur compositing. This keeps the "looks native" property a compositor-layer guarantee rather than something every AUIL-authored surface has to reimplement, consistent with the design intent in `auil-asl-spec.md` §3.2.

---

## 5. Failure / degraded-mode behavior

- If the UI Runtime crashes or isn't yet started (very early boot, or Agent Core/local-model still warm-loading per `agent-core-spec.md` §8), the compositor continues running and displays whatever surface is currently mapped — most commonly, nothing but a background, or the Fallback Shell's own minimal client (`fallback-shell-spec.md`) if the UI Runtime is confirmed down rather than just not-yet-started. The compositor itself has no "agent unavailable" logic; that decision and indicator belong entirely to the Fallback Shell / UI Runtime, per parent §3.7.
- The compositor is not on the parent doc's protected-unit list by name, but restarting it mid-session is disruptive enough (it owns every visible surface) that it's a reasonable candidate for that list — this is flagged as an open item (§7) for the Policy Broker's protected-unit configuration rather than decided here.

---

## 6. Security summary

| Threat | Mitigation |
|---|---|
| A malicious client binds the confirmation-surface role to spoof a legitimate confirmation prompt | Role is granted to exactly one fixed, credential-verified client identity at compositor startup, never re-negotiated at runtime (§3) |
| A UI-Runtime-rendered element visually overlaps a real confirmation surface to trick the user into misclicking | Exclusive input routing and top-most compositing while the confirmation surface is mapped means no other surface can receive input or render above it during that window (§3) |
| Legacy XWayland app used as a side-channel to synthesize fake input toward the confirmation surface | Input exclusivity while a confirmation surface is mapped applies to every other client uniformly, XWayland included — there is no privileged input path for legacy apps |

---

## 7. Open items before implementation

1. **`confirmation-surface-v1` protocol spec** — this document describes the required behavior; the actual Wayland protocol XML and a wlroots implementation patch still need to be written.
2. **Blur-region protocol choice** (§4) — whether an existing stock protocol is adequate or a small custom one is needed; needs an actual survey of current wlroots protocol support before deciding.
3. **Compositor's place on the protected-unit list** — flagged in §5; needs a decision made alongside the Policy Broker's protected-unit configuration, not unilaterally here.
4. **External-surface embedding for legacy apps** (§2) — the exact mechanism by which the UI Runtime gives a legacy Wayland/XWayland surface a slot inside an AUIL tree needs its own small spec, likely as an addendum to `auil-asl-spec.md` rather than this document.