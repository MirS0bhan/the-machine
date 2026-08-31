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
| L5 | Wayland Compositor | **Partial** | Framebuffer / DRM present; Wayland SHM + xdg-shell (G17 in #215). wlroots/XWayland are non-goals. |
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
