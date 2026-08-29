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
