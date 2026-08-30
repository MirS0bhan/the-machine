# Intent Routing

An intent event (`04-events/01-event-model.md` §2) is a binding, not a function call the AUIL author has to know the destination of. This document covers what the three reference sigils actually resolve to, how an `mcp:` target gets a handler, and — the point most relevant to the visual language specifically — why none of this is ever visible in the UI as a difference in how a binding is authored.

## 1. The three sigils, compared

| Sigil | Direction | Cardinality | Typical prop |
|---|---|---|---|
| `$lambda:path` | One-way, continuous | A running lambda's output streamed into the node | `media`'s `src=` |
| `mcp:method` | Fire-once, on the named event | An intent, dispatched once per event occurrence | `on:press=`, `on:change=` |
| `@path` | Two-way where the primitive supports it, otherwise read-only | A State Store path, live-subscribed | `field`'s `value=`, `slider`'s `value=`, `chart`'s `data=` |

## 2. How an `mcp:` target resolves

`method` names follow a `namespace.verb` convention (`video_player.play`, `app.confirm`, `ui.status`) that mirrors the MCP Bus's own registry namespaces (`docs/architecture/layers.md` §3: `mcp-intent`, `event-handler`, `system-op`, `state-op`) — resolution is an O(1) prefix lookup, not a search. From an AUIL author's point of view, there is exactly one thing to get right: pick a name that reads as `thing.action`, and bind it. What happens after that dispatch is out of scope for the node that dispatched it:

1. If a lambda is already registered for that method, it handles the call directly and (usually) returns fast enough that no `loading` state is visually necessary.
2. If nothing is registered, the call wakes the Agent Core (`docs/architecture/layers.md` §1.3's routing rule — no handler exists → wake the agent), which reasons about the request, possibly escalating to the cloud tier, and eventually either returns a result or emits its own `lambda.register` for that method going forward.

**The binding itself never changes between these two cases.** This is the point worth stating plainly: `on:press=mcp:video_player.play` is exactly the same AUIL whether it's the first time anyone has ever asked to play a video (a multi-second cloud-reasoned round trip the first time) or the thousandth (an instant, already-registered lambda call). The system's "retire early, retire often" behavior (`docs/architecture/philosophy.md` commitment 5) is invisible at the authoring layer by design — an agent composing a screen never has to know or care whether a given intent is currently agent-reasoned or already-deterministic, and neither does this document set's guidance change based on it.

## 3. What the UI shows while an intent is pending

A node whose bound intent hasn't resolved yet MAY transition to `state:loading` (`03-widgets-and-types/03-states-and-variants.md` §2) — but only if the wait is long enough to be worth communicating; per Principle 4, the local motion response to the triggering event has already happened regardless. There is no different visual treatment for "this is waking the agent" vs. "this is calling an already-registered lambda" — from the person's point of view those are both just "the system is working on it," and inventing a visual distinction between them would expose an implementation detail nobody asked for and violate Principle 1.

## 4. Escalation and privacy are not hidden, even though routing is invisible

§2 said routing (local lambda vs. agent-reasoned) is invisible in the UI, and that stands. It does not follow that the local/cloud *processing locus* is hidden — `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §5 defines a small, honest, persistent indicator of whether the agent's current reasoning is happening on-device or in the cloud, independent of whether any particular button press happens to be lambda-fast or agent-reasoned. The distinction that matters for transparency is "where is my data going," not "is this a cached decision or a fresh one" — the first is load-bearing for trust, the second genuinely isn't the person's problem.

## 5. Errors and denial

An `mcp:` call that resolves to a Policy Broker `DENY` returns through the same path a successful call would, and the node's bound handler is responsible for reflecting that as a `state:error` transition with real, specific copy (`01-hig/03-content-and-voice.md` §4) — not a generic failure state. A call that resolves to `CONFIRM` does not put the *triggering* node into any special state at all; the Confirmation Surface renders entirely outside the agent's own UI tree, per `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §2, and the triggering node simply waits (optionally showing an inert, non-interactive "waiting for confirmation" status) until the Broker's decision comes back.

---

*Cross-references: `04-events/01-event-model.md` (the events that trigger intents), `03-widgets-and-types/03-states-and-variants.md` (the `loading`/`error` states referenced here), `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §2, §5 (the Confirmation Surface and the processing-locus indicator), `docs/architecture/layers.md` §1.3, §3 (MCP Bus registry and routing), `docs/architecture/philosophy.md` commitment 5 ("retire early, retire often").*
