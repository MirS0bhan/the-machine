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
