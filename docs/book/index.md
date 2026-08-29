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

Each implemented component is a Python package with its own `pyproject.toml` and an
MCP server entry point. From the repository root:

```bash
cd lambda-server && uv run python -m lambda_server.server      # HTTP + MCP API
cd state-store    && uv run python -m state_store.mcp_server
cd policy-broker  && uv run python -m policy_broker.mcp_server
cd event-bus      && uv run python -m event_bus.mcp_server
cd agent-core     && cargo run --bin agent-core
cd local-model    && uv run python -m local_model.mcp_server
cd ui-engine      && uv run python -m ui_engine.server
```

### Running the tests

```bash
uv run pytest lambda-server/test_http_api.py     # 9/9 HTTP API tests
uv run pytest ui-engine-demo/test_demo.py         # UI Engine demo tests
uv run pytest state-store/tests event-bus policy-broker agent local-model
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
| L1 | Event/Scheduler Bus | **Implemented** | MCP server, router, event taxonomy, scheduling |
| L2 | Policy Broker | **Implemented** | Rule interpreter, audit log, decision engine, MCP server |
| L4 | Agent Core | **Implemented** | Session loop, hybrid router, privacy gate, systemd control, skills |
| L4 | Local Model Interface | **Implemented** | Engine, privacy tagging, embedding backend, MCP server |
| L5 | UI Engine | **Implemented** | AUIL/ASL parser, runtime, patch protocol, renderer, models |
| L5 | UI Engine Demo | **Implemented** | Terminal `AbstractRenderer`, `demo.auil`, input loop, tests |
| L3 | MCP Bus | **Implemented** | Rust daemon with method registry + socket forwarding |
| L0 | System Daemon | **Implemented** | Rust daemon (mock kernel ops for dev) |
| L5 | Wayland Compositor | **Partial** | Rust logical model; wlroots integration planned |
| L3.7 | Fallback Shell | **Implemented** | Rust console recovery mode |

The architecture, broker, state store, event bus, agent core, local model, and UI
engine specs are carried forward verbatim in their chapters with live implementation
references appended. The MCP Bus, System Daemon, Compositor, and Fallback Shell remain
design-only and are marked as such in their chapters.

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
├── docs/                     # this documentation set
│   ├── spec.md               # architecture definition
│   ├── *-spec.md             # component design specs
│   ├── book/                 # expanded narrative + demo chapter
│   ├── build.py              # documentation generator
│   └── build/                # generated output (index.html, book.md)
├── lambda-server/            # L1 sandboxed function runtime (HTTP + MCP)
├── state-store/              # L1 UI + system state, pub/sub
├── event-bus/                # L1 reactive routing + scheduler
├── policy-broker/            # L2 capability enforcement + audit
├── agent/                    # L4 hybrid LLM router + session loop
├── local-model/              # L4 Tier-A local inference + privacy
├── ui-engine/                # L5 AUIL/ASL parser, runtime, patch protocol
├── ui-engine-demo/           # L5 terminal demo app
└── Makefile                  # `make docs`, `make serve`, `make clean`
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
