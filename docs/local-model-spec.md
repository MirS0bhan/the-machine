# Local Model Interface — Always-On Tier A Runtime, Privacy Tagging & Embedding Backend

**Fills:** part of §7.4 of `agent-native-os-architecture.md` ("Local/cloud routing thresholds") and the Tier A half of §3.5/§6
**Related:** `agent-core-spec.md` §3–§4 (the router this component serves), `lambda-server-spec.md` §10.2 (open item: semantic search needs an embedding backend), `policy-broker-spec.md` §11 (capability validation for this component's own manifest)
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation

---

## 0. Design goals

1. **Always resident, never the bottleneck.** This is the one model in the system that must be loaded before the OS is usable at all (per `agent-core-spec.md` §8, boot blocks on this). Every design choice here favors low, predictable latency over raw capability.
2. **The privacy boundary lives here, not just in the router.** `agent-core-spec.md` §4 gates *outbound* cloud calls on a `privacy_tag`; this spec is where that tag actually gets produced, at the point content first enters the model layer — the earliest possible point, so nothing downstream has to reconstruct "was this sensitive" from context.
3. **A general-purpose service, not just the Agent Core's private tool.** Other components need small, cheap inference-adjacent primitives — the Lambda Server's semantic search (`lambda-server-spec.md` §10.2) is the clearest existing example — and shouldn't have to become their own bootstrapping problem to get an embedding. This component exposes itself over MCP like everything else, so anything capability-scoped to call it, can.
4. **Degrades to "absent," not "wrong."** If this component fails to load or crashes, the system's answer is the Fallback Shell (parent §3.7), not a silently degraded model giving worse answers. Health is binary from the outside: ready, or not.

---

## 1. Component overview

- A single resident process managing one loaded model (candidate class: small, quantized, few-billion-parameter, per parent §6) via a local inference runtime (e.g. a llama.cpp-style engine) — this spec does not mandate a specific engine, only the MCP-facing contract around it.
- Owns model lifecycle: load at boot (blocking `agent-core-spec.md`'s readiness signal), health reporting to the Event Bus (`event-bus-spec.md` §1, `category: health`), and — if ever needed for resource pressure reasons — controlled unload/reload, gated the same as any protected-unit action if it's ever promoted to that list.
- Runs as its own sandboxed process under the Lambda Server's general process model (`lambda-server-spec.md` §3), but is treated as a **standing, not warm-pool-evictable** service — it does not compete for warm-pool slots the way an ordinary lambda might, since the Agent Core has no fallback if it's evicted.

---

## 2. MCP surface

| Tool | Purpose |
|---|---|
| `localmodel.complete(context, options)` | Text completion / chat-style inference. `options` includes a `privacy_tag` **input** hint (see §3) that the caller may set explicitly, in addition to whatever the model layer infers. |
| `localmodel.classify_intent(input, categories?)` | Cheap structured classification (new-task vs. continuation, category taxonomy from `event-bus-spec.md` §1) — the primitive `agent-core-spec.md` §3's routing decision is actually built from. |
| `localmodel.embed(text)` | Returns a fixed-dimension embedding vector — the backend `lambda-server-spec.md` §10.2 flagged as an open bootstrapping problem for `lambda.search` ranking (§4 below). |
| `localmodel.health()` | Load state, resource usage, last-inference latency — feeds the Event Bus health category. |
| `localmodel.reload(model_version?)` | Swap the loaded model artifact; blocks new inference calls until complete, existing in-flight calls finish against the old weights. |

All calls are capability-gated like any MCP surface — `CAP_IPC_CALL(targets=[localmodel])` must be declared by any caller other than the Agent Core, so a rogue lambda can't quietly use this as a free inference backend outside what its manifest describes.

---

## 3. Privacy tagging — where the hard rule actually gets enforced

- Any `localmodel.complete`/`.classify_intent` call whose input context was sourced from a `CAP_MIC`, `CAP_CAMERA`, or a `CAP_FS_READ`-scoped personal path (as declared in the *caller's own* capability grant, `lambda-server-spec.md` §2.1) causes this component to stamp the **output** context object with `privacy_tag: true` before returning it.
- This tag is not a suggestion carried in a text field the Agent Core's prompt could rationalize away — it's a structured field on the MCP response object that `agent-core-spec.md` §4's compiled gate checks mechanically before permitting any Tier B call built from that context. The Local Model Interface is upstream of that gate; it's the component responsible for the tag existing at all, correctly, at the earliest point.
- Tagging is **sticky and monotonic through a turn**: once any piece of a wake's gathered context is tagged, the entire outbound plan for that wake is treated as tagged, even if other parts of the context were untagged — this is the conservative direction to err in, per the parent doc's own framing of this as a hard rule, not a judgment call.
- This component does not itself decide whether cloud escalation is *allowed* — that's `agent-core-spec.md` §4's job. It only guarantees the input to that decision is accurate.

---

## 4. Embedding backend for `lambda.search`

`lambda-server-spec.md` §10.2 identified a bootstrapping problem: better-than-keyword search over the function registry needs embeddings, but running that as its own lambda creates a circular dependency (the Lambda Server would depend on a lambda to search lambdas). This spec resolves it directly: `localmodel.embed` is a **standing, boot-time-available** service, not a lambda — the Lambda Server's registry search calls `localmodel.embed` on function descriptions at `lambda.register` time (to index) and on the search query at `lambda.search` time (to rank), with no circularity, because this component is up before the Lambda Server ever needs to serve a search.

---

## 5. Resource management

- Fixed memory/compute budget reserved at boot, sized so the model stays resident without contending with Lambda Server warm pools for the same resource envelope — this is a system-image sizing decision (how big a model ships by default), not something this spec's runtime negotiates dynamically.
- `localmodel.reload` (§2) exists for two cases: a model update shipped as part of an OS update, or a user-configured swap to a different local model size/quality tradeoff — both are deliberate, infrequent, human-or-update-triggered events, not something the Agent Core does routinely as part of its own reasoning.

---

## 6. Degraded-mode contribution

- Per parent §3.7, the Fallback Shell must work with **zero agent involvement**, which structurally means zero dependency on this component too. This spec's failure mode is therefore designed to be loud and immediate: `localmodel.health()` reporting not-ready is what `agent-core-spec.md` §10 treats as the primary trigger for the Fallback Shell to take over, not a timeout guess.
- There is no "cloud model substitutes for local model during an outage" fallback — per the parent doc, Tier B is a planning resource invoked *by* Tier A, not a replacement *for* Tier A, and privacy-tagged interactions specifically must never fail over to cloud just because local is down. An outage of this component is an outage of interactive intelligence, full stop, and the system's answer to that is the deterministic Fallback Shell, not a workaround.

---

## 7. Security summary

| Threat | Mitigation |
|---|---|
| Privacy tag omitted or falsified, letting sensitive content reach the cloud | Tag is computed here, at the model layer, from the caller's own declared capability grant — not from caller-supplied metadata that could be spoofed (§3) |
| Unauthorized lambda uses this component as a free inference/embedding backend | Standard `CAP_IPC_CALL` gating, same as any MCP target (§2) |
| Model reload swaps in a tampered artifact | Mirrors `lambda-server-spec.md` §9's compiled-artifact-tampering mitigation: artifact hash verified at load, same pattern reused here rather than invented fresh |
| Resource exhaustion from unbounded `localmodel.embed` calls (e.g. indexing storm) | Broker-enforced per-caller rate limiting, same mechanism as any other MCP surface (`policy-broker-spec.md` §6) |

---

## 8. Open items before implementation

1. **Model candidate selection** — parent §6 says "few-billion-parameter range" as a placeholder; needs an actual choice weighed against the boot-blocking latency budget (`agent-core-spec.md` §8).
2. **Embedding dimensionality/versioning** — if a `localmodel.reload` swaps to a model with a different embedding space, every existing `lambda.search` index entry is stale; needs a re-indexing trigger, not silent degradation.
3. **Privacy tag granularity** — §3 currently tags at the whole-context level; a finer-grained per-span tag might let Tier A safely escalate the *untagged* portion of a mixed context to cloud instead of blocking the whole turn, but that's a materially bigger design (and risk surface) than this version takes on.
4. **Classification taxonomy versioning** — `localmodel.classify_intent`'s `categories` schema needs to stay in lockstep with the Event Bus's event taxonomy (`event-bus-spec.md` §1) without the two documents drifting independently.
5. **Concurrent request handling** — whether this component serializes inference calls (simplest, but a slow embed job could stall an interactive completion) or needs priority scheduling between interactive (Tier A routing) and background (`lambda.search` indexing) callers.
