# Agent-Native OS — Architecture Definition
 
**Codename:** (unnamed)
**Version:** 0.1 (design draft)
**Status:** Conceptual architecture, pre-implementation
 
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

