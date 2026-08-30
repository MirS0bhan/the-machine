# The Machine — Documentation

**Version:** 0.1  
**Status:** Rust boot daemons (Phases 1–7) + Python MCP reference servers for tests

Welcome to the documentation for **The Machine**, an agent-native operating system where a single AI agent sits between human intent and system mechanisms.

---

## Quick Links

- [Architecture Overview](./architecture/overview.md) — High-level system design
- [Runtime Model](./architecture/runtime-model.md) — Agent→MCP→UI loop (implemented)
- [Expansion Proposal](./architecture/expansion-proposal.md) — Roadmap to fully agentic OS
- [Gap Analysis](./architecture/gap-analysis.md) — Living checklist vs north-star
- [Component Inventory](./reference/component-inventory.yaml) — Canonical service list (verified by CI)
- [Design Philosophy](./architecture/philosophy.md) — Core principles and commitments
- [Layer Reference](./architecture/layers.md) — Detailed layer-by-layer breakdown
- [Component Reference](./components/) — Each component's spec
- [Getting Started](./guides/getting-started.md) — How to build and run
- [Bare-Metal Desktop](./guides/bare-metal.md) — Install on physical hardware
- [Testing & Coverage](./guides/testing.md) — Test suites and coverage reporting
- [Python ↔ Rust Overlap](./guides/python-rust-overlap.md) — Which implementation to use
- [Glossary](./reference/glossary.md) — Terminology reference

---

## What Is The Machine?

Traditional operating systems separate *mechanism* (kernel, drivers, IPC) from *policy* (window managers, app frameworks, user intent), and every layer in between exists to let humans manually wire mechanism to policy: file managers, launchers, app stores, config files.

This OS removes the manual wiring. A single **Agent Core** sits between the human's intent and the system's mechanisms. The human states what they want; the agent decides which system capabilities to invoke and what UI should exist to reflect that. Everything else — kernel, compositor, sandboxed execution — exists to give the agent a **safe, fast, auditable surface** to act on.

---

## Core Components

| Layer | Component | Responsibility |
|-------|-----------|----------------|
| L0 | [System Daemon](./components/system-daemon.md) | evdev input, kernel-parameter MCP surface |
| L1 | [State Store](./components/state-store.md) | Persistent UI/system state, sled + subscriptions |
| L1 | [Event Bus](./components/event-bus.md) | Reactive routing, timers, D-Bus/fs/audio adapters |
| L1 | [Lambda Server](./components/lambda-server.md) | Sandboxed function execution, synthesis |
| L2 | [Policy Broker](./components/policy-broker.md) | Capability enforcement, confirmation, audit |
| L3 | [MCP Bus](./components/mcp-bus.md) | Message fabric, intent registry, policy middleware |
| L4 | [Agent Core](./components/agent-core.md) | Hybrid LLM router, session loop, skills |
| L4 | Local Model Daemon | GGUF inference (`local-model-daemon`) |
| L4 | Marketplace | Curated bundle install (`marketplace`) |
| L5 | [UI Runtime](./components/ui-runtime.md) | Declarative renderer, patch protocol |
| L5 | [Compositor](./components/compositor.md) | Framebuffer compositor, confirmation surface |
| L5 | [Fallback Shell](./components/fallback-shell.md) | Zero-inference recovery UI |

---

## Key Design Principles

1. **Agent decides *what*, never *how* at the low level** — it orchestrates vetted, sandboxed primitives
2. **Real-time paths never touch inference** — keystrokes, mouse movement, audio/video frames flow through deterministic code
3. **Everything speaks MCP** — one protocol, one audit format, one place to enforce policy
4. **Deny by default** — Policy Broker is the single gatekeeper, not the agent's judgment
5. **Retire early, retire often** — the agent makes itself unnecessary for intent families as fast as possible

---

## Getting Started

1. Install dependencies — see [Getting Started](./guides/getting-started.md)
2. Build: `make build`
3. Test: `make test-all`
4. Verify docs ↔ code: `make verify-docs`
5. Build ISO: `make iso`
6. Bare metal: see [Bare-Metal Desktop](./guides/bare-metal.md)
7. Build documentation: `make docs`

See the [Getting Started Guide](./guides/getting-started.md) for detailed setup.

---

## License

Proprietary — all rights reserved.
