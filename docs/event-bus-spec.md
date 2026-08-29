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
