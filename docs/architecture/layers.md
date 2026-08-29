# Layer Reference

This document provides a detailed description of each layer in The Machine's architecture, from kernel to human.

---

## Layer 0: Kernel & I/O Subsystem

**What it is:** A standard Linux or BSD kernel, unmodified, plus normal drivers (GPU, audio, input, network, storage).

**Role:** Pure mechanism. It knows nothing about agents or intent. It exposes standard syscalls, device nodes, DRM/KMS for GPU, ALSA/PipeWire for audio, evdev/libinput for input, and the network stack.

**Key component:** [System Daemon](../components/system-daemon.md)

**Boundary rule:** The kernel is *never* addressed directly by the Agent Core. All kernel-parameter changes go through the System Daemon, which exposes a narrow, schema-validated subset of operations over MCP.

---

## Layer 1: Lambda Server, State Store, Event Bus

This layer turns the agent's decisions into running software. It consists of three sub-components.

### 1.1 Lambda Server

**What it is:** A local (with optional cloud burst) serverless runtime. The agent deploys, updates, and invokes small sandboxed functions here.

**Key properties:**
- **Warm pools** for persistent functions; ephemeral containers for one-shot tasks.
- **Glue, not reinvention:** functions call into a vetted base image (ffmpeg, headless browser, etc.).
- **Sandbox:** OCI containers or microVMs with seccomp + namespaces.
- **Versioning:** every deploy is immutable; automatic rollback on crash-loop.

**Key component:** [Lambda Server](../components/lambda-server.md)

### 1.2 State Store

**What it is:** A persistent, structured store for UI state and system/task state.

**Data model:** Single hierarchical store with dot-separated path addressing and four top-level namespaces:
- `ui.<tree>` — UI trees (node-addressable)
- `task.*` — tasks, session metadata, intent history
- `prefs.*` — user preferences
- `perm.*` — permission-grant records (Broker-only writes)

**Patch model:** Every write is internally a patch (old → new), with a global, monotonic revision number. Subscriptions (`state.watch`) yield patch events.

**Key component:** [State Store](../components/state-store.md)

### 1.3 Event/Scheduler Bus

**What it is:** An async event bus that lets the system be reactive, not strictly turn-based.

**Role:** Decides *when* the Agent Core needs to be invoked at all. Most events are handled entirely inside L1/L0 without ever reaching the agent. Only events that require a *decision* get routed up to L4.

**Routing:**
- If a registered lambda handles the event pattern → deliver directly
- If no handler exists → wake the Agent Core
- Once the agent registers a handler, the bus routes subsequent events directly

**Scheduler:** `CAP_TIMER` grants allow lambdas to schedule one-shot or recurring events.

**Key component:** [Event Bus](../components/event-bus.md)

---

## Layer 2: Policy Broker

**What it is:** The single most important safety component. A small, deterministic (non-LLM), formally-scoped service that mediates *everything* the Agent Core wants to do to the system.

**Responsibilities:**
- **Capability grants:** approves, denies, or asks for confirmation for every lambda capability request
- **Schema validation:** rejects free-form kernel/system writes
- **Rate limiting & anomaly detection:** holds suspicious requests
- **Audit log:** immutable, queryable log of every MCP call
- **Prompt-injection containment:** treats ingested content as data, never as instructions

**Decision outcomes:**
- `ALLOW` — request proceeds with a signed grant token
- `DENY` — request rejected with a structured reason
- `CONFIRM` — requires explicit, out-of-band human approval
- `HOLD` — queued for anomaly review; auto-times-out to `DENY`

**Key component:** [Policy Broker](../components/policy-broker.md)

---

## Layer 3: MCP Bus

**What it is:** The uniform protocol connecting every layer. Not just "how the agent talks to tools" — MCP *is* the system bus.

**Why this matters:**
- Kernel parameter changes → MCP call to System Daemon (via Broker)
- Lambda deploy/invoke → MCP call to Lambda Server (via Broker)
- UI updates → MCP-shaped patches to the State Store
- Inter-lambda communication → MCP messages

**One protocol, one audit format, one place to enforce policy.**

**Registry:** A single registry with four namespaces:
- `mcp-intent` — registered via `exposes_mcp` (lambda methods)
- `event-handler` — registered via `handles_event`
- `system-op` — fixed, shipped with the OS image
- `state-op` — fixed, always resolves to State Store

**Resolution:** O(1) lookup — extract namespace from method prefix, look up the handler, forward.

**Key component:** [MCP Bus](../components/mcp-bus.md)

---

## Layer 4: Agent Core

**What it is:** The decision-making brain. A **router + two-tier model strategy**:

**Tier A — Local model (small, on-device):**
- Runs at all times, low latency (tens of ms), no network dependency
- Handles: intent classification, routine UI patches, simple tasks
- Also acts as the **first-pass filter** deciding whether a request needs the cloud model

**Tier B — Cloud model (large, frontier-scale):**
- Invoked only when Tier A flags genuine complexity
- Returns a structured plan (function specs, UI patch intents)
- Never in the real-time loop

**Routing logic:**
1. Local model classifies intent + estimates complexity/novelty
2. If known task pattern and low ambiguity → local model handles directly
3. If privacy-sensitive → local model handles, cloud excluded by structural gate
4. Else → escalate to cloud model

**What the Agent Core is *not* allowed to do:**
- Directly touch the kernel, raw devices, or filesystem
- Write and execute low-level unsandboxed code
- Grant itself capabilities

**Key component:** [Agent Core](../components/agent-core.md)

---

## Layer 5: UI Runtime & Compositor

### 5.1 Wayland Compositor

**What it is:** A standard-ish Wayland compositor (based on wlroots) so that conventional Wayland/X11 clients can still run.

**Custom extension:** `confirmation-surface-v1` — a reserved role that only the Policy Broker's Confirmation Surface Daemon may bind. While mapped, it renders above all other surfaces, routes all input exclusively to it, and is unfakeable by any other client.

**Key component:** [Compositor](../components/compositor.md)

### 5.2 Declarative UI Runtime (AUIL/ASL)

**What it is:** A renderer that consumes the **UI State Tree** (from the State Store) and draws it — conceptually similar to a React renderer consuming a virtual DOM.

**Design requirements:**
- JSON/schema-based, not a general-purpose programming language
- Small, fixed set of primitives: containers, text, media surface, input field, list, button, chart
- Every component has a declared data/event binding back to MCP
- Diffable: the agent emits *patches* to the tree, not full re-renders

**Patch protocol (five ops):**
- `~id(props)` — update properties
- `+anchor: node` — insert a new node
- `-id` — remove a node and its descendants
- `!id: node` — replace a subtree wholesale
- `@id → other-id` — move a subtree

**ASL (Adaptive Style Language):**
- Token-based styling with a fixed token palette
- State-driven transitions (`state:loading` binds to a state.watch)
- No custom CSS — everything is a token reference or a fixed layout property

**Key component:** [UI Runtime](../components/ui-runtime.md)

### 5.3 Fallback Shell

**What it is:** A minimal, fully deterministic UI and control layer that works with **zero agent involvement**.

**Two operating modes:**
1. **Frozen last-good view** — static snapshot of the last committed UI tree, read-only
2. **Recovery console** — fixed action set: view status, view logs, restart agent, connect network, safe terminal

**Trigger conditions:**
- Boot-time: before Agent Core readiness signal
- Runtime: UI Runtime crash, Agent Core unreachable
- Explicit: safe mode key combination

**Key component:** [Fallback Shell](../components/fallback-shell.md)

---

## Layer 6: Human

**What it is:** The user. They interact with the system through natural language (text/voice) and through the declarative UI that the agent synthesizes.

**Interaction modes:**
- **Text input:** typed or dictated
- **Voice input:** transcribed by the local model or a dedicated speech-to-text lambda
- **UI gestures:** clicking, scrolling, selecting — these are interpreted as events and routed through the Event Bus
- **Confirmation:** the user approves or denies sensitive operations via the Confirmation Surface

**The human is not expected to:**
- Navigate a filesystem
- Configure system settings manually
- Install or manage applications
- Troubleshoot low-level issues (the Fallback Shell is a safety net, not the primary interface)
