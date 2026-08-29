# Design Philosophy

This document describes the core principles and design commitments that shape every component of The Machine.

---

## 1. The Agent Is the Interface

Traditional operating systems present the human with a **direct manipulation** interface: windows, icons, menus, and a mouse. The human has to learn the system's conventions, navigate its hierarchy, and perform all the wiring themselves.

The Machine inverts this: the human states **what they want**, and the agent figures out **how to make it happen**. The UI is not a canvas for manual control — it's a **mirror** of the agent's understanding, showing state, offering context, and accepting natural input.

**Implication:** No part of the system is designed to be operated by a human directly. The human interacts through natural language (text/voice) and through the declarative UI that the agent synthesizes.

---

## 2. Mechanism vs. Policy — Separation at the Right Boundary

Traditional systems separate mechanism and policy at the **kernel vs. userspace** boundary. The kernel provides mechanisms (syscalls, IPC, scheduling) and userspace implements policy (window management, application logic, user preferences).

The Machine moves this separation higher: **the Policy Broker** is the mechanism, and **the Agent Core** is the policy. The Broker offers a safe, auditable MCP surface; the Agent decides what to call on that surface.

**Implication:** The Broker is deterministic, minimal, and formally gate-checkable. The Agent is probabilistic, generative, and the only place where novel reasoning happens. They don't blur.

---

## 3. Real-Time Paths Are Deterministic

Keystrokes, mouse movements, audio buffers, and video frames flow through **deterministic, non-LLM code**. The agent is invoked only at *decision points* — new intents, ambiguity, state transitions — not per-frame or per-keystroke.

**Why:** Inference is too slow and too unpredictable for real-time I/O. If the agent were in the critical path, the system would feel sluggish and unreliable. The machine should feel as responsive as a conventional OS, even when the agent is thinking.

**Implication:** The Event Bus routes most events to local handlers (lambdas) without waking the agent. The compositor and UI Runtime are entirely deterministic. The agent is a decision-maker, not a real-time processor.

---

## 4. The Agent Decides *What*, Never *How*

The agent never gets raw root access, never writes kernel code by hand, never re-implements codecs. It orchestrates **vetted, sandboxed primitives**:

- **Sandboxed lambdas** (OCI containers) execute the code the agent writes
- **Pre-approved base images** (ffmpeg, a headless browser, codec libraries) provide the "how"
- **Policy Broker** mediates every capability request

**Why:** If the agent could execute arbitrary low-level code, a single hallucination could destroy the system. By restricting the agent to orchestrating sandboxed primitives, the system is safe by construction.

**Implication:** The agent writes *orchestration code*, not *low-level implementation*. It calls existing libraries, combines them, and registers new lambdas that use them. It never implements crypto, decoders, or kernel logic from scratch.

---

## 5. Retire Early, Retire Often

The agent's long-term goal is to **make itself unnecessary** for a given intent family. When the agent synthesizes a capability for the first time, it registers a lambda that handles that intent directly. On the next occurrence, the Event Bus routes the event to the lambda without waking the agent.

**Why:** Inference is expensive. The more the agent can delegate to deterministic code, the faster, cheaper, and more reliable the system becomes. The agent should be a *teacher* that creates specialized students (lambdas) and then steps back.

**Implication:** The agent's workload shrinks over time. Common tasks become instant, deterministic, and agent-free. The system gets faster the more it's used.

---

## 6. Deny by Default, Explain on Denial

The Policy Broker is the single gatekeeper. Every capability request is checked against a set of rules. The default outcome is `DENY`. Every rejection carries a **machine-readable reason** (which rule fired, what was missing).

**Why:** The agent is probabilistic and can be tricked (prompt injection, hallucination). The Broker is deterministic and formal. The Broker is the system's immune system — it does not trust the agent, it checks every action.

**Implication:** The agent can self-correct without a human in the loop for common denials, because it gets structured feedback. But a human is still required for sensitive operations (confirmed via the Confirmation Surface).

---

## 7. Everything Speaks MCP

MCP (Model Context Protocol) is the one protocol that every component uses to communicate. There is no other IPC. No component bypasses the bus, including the Agent Core.

**Why:** One protocol means one audit format, one place to enforce policy, one place to log every action. It also means the same "tool-calling" muscle the LLM already has is the *native* language of the whole OS.

**Implication:** The system is transparent and auditable end-to-end. Every action the agent takes is a logged MCP call. The Event Bus, Lambda Server, State Store, and System Daemon all speak the same protocol.

---

## 8. Privacy Is Structural, Not Aspirational

Privacy-sensitive inputs (mic, camera, personal files) are **routed to the local model by default**. Cloud escalation for such content requires explicit user opt-in per session or per task, and is gated by a structural check at the routing layer — not by the model's judgment.

**Why:** If the model decides whether data is "sensitive enough" to send to the cloud, a hallucination or prompt injection could leak private data. By making the check structural (a compiled `privacy_tag` check in the routing logic), the system is safe by construction.

**Implication:** The local model produces a `privacy_tag` for every input. If the tag is `true`, the cloud model is structurally excluded from the routing path. No amount of prompt engineering can bypass this.

---

## 9. Resilience Without Inference

The machine must be usable **when inference is unavailable**:

- Before local model loads (cold boot)
- If local model crashes
- If cloud is unreachable
- If the Agent Core itself crashes

The **Fallback Shell** is a minimal, fully deterministic UI that works with zero inference. It shows a frozen view of the last good UI state and offers a recovery console.

**Why:** A computer that requires inference to boot is a computer that can't be trusted. The Fallback Shell ensures the machine is always usable enough to see status, restart components, and recover.

**Implication:** The Fallback Shell is the system's dependency anchor. It depends on almost nothing — no model, no Agent Core, no UI Runtime. It is a static binary that reads the State Store directly and renders a simple UI.

---

## 10. Confirmation the Agent Cannot Forge

When the Policy Broker requires human confirmation for a sensitive operation (e.g., "allow camera access", "restart the agent", "format a disk"), it renders a confirmation dialog on a **reserved compositor surface that the agent cannot bind to**.

The dialog content is **Broker-authored** (not agent-authored), so the agent cannot inject misleading markup or trick the user into approving something they didn't intend.

**Why:** If the agent could compose the confirmation dialog, it could lie. It could say "allow microphone access" when the actual request is "format the disk." The Broker removes this attack vector by owning the confirmation UI itself.

**Implication:** The Confirmation Surface is a small, well-defined extension to the Wayland compositor that only the Broker may bind. It is the system's "trusted path."

---

## Summary: Ten Commitments

| # | Commitment |
|---|------------|
| 1 | The agent is the interface |
| 2 | Mechanism vs. policy separated at the right boundary |
| 3 | Real-time paths are deterministic |
| 4 | The agent decides *what*, never *how* |
| 5 | Retire early, retire often |
| 6 | Deny by default, explain on denial |
| 7 | Everything speaks MCP |
| 8 | Privacy is structural, not aspirational |
| 9 | Resilience without inference |
| 10 | Confirmation the agent cannot forge |
