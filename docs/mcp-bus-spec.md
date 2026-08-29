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

## 8. Open items before implementation

1. **Wire protocol between Bus and components** — likely reuses the same length-prefixed framing `lambda-server-spec.md` §10.1 proposes for its own IPC layer, but this needs to be the *same* choice, not an independently-made one, since components speak both.
2. **Namespace prefix collision rules** — the "extract namespace from method prefix" resolution (§2) needs a formal grammar so `mcp-intent` keys can never accidentally shadow a `system-op`/`state-op` prefix.
3. **Registry persistence across Bus restarts** — is the registry rebuilt from each component's own manifest on Bus restart (source-of-truth stays distributed), or does the Bus persist its own copy (faster cold start, but a second place truth can drift)?
4. **Multi-instance / sharding** — out of scope for a single-user machine today, but worth flagging that this design assumes exactly one Bus instance; nothing here has been designed for horizontal scaling.
