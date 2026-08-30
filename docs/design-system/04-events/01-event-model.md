# Event Model

Principle 4 (`01-hig/01-design-principles.md`) draws a hard line between what must never wait on the MCP bus and what may. This document is the precise mechanics of that line: the fixed event vocabulary, what each event does, and — the part that's easy to get subtly wrong — the fact that a single physical event can have two independent consequences at once.

## 1. The fixed event vocabulary

| Event | Kind | Crosses MCP? | Notes |
|---|---|---|---|
| `hover` | Input-triggered | Never | Pointer enters/exits the hit area |
| `press` | Input-triggered | Only if the node has an `on:press=mcp:...`/`$lambda:...` binding | See §2 — this is the event with two independent consequences |
| `release` | Input-triggered | Never | Pointer/keyboard activation ends without completing a press (e.g. pointer dragged off the target before release) |
| `focus` | Input-triggered | Never | Keyboard or assistive-technology focus lands on the node |
| `blur` | Input-triggered | Never | Focus leaves the node |
| `drag` | Input-triggered | Never for the motion itself; a drag's *completion* (a reorder, a value commit) is a `change` | Continuous pointer motion while a press is held on a draggable node |
| `change` | Input-triggered, but always touches the State Store | If the node also has an `on:change=mcp:...` binding | A value commit — a `field`'s text, a `slider`'s value, a `toggle`'s checked state |
| `loading` | Data-bound | N/A — this *is* the visual reflection of something already in flight over MCP, not a trigger for a new call | Declared via `state:loading →`, not `on:` |
| `error` | Data-bound | N/A, same reasoning as `loading` | Declared via `state:error →` |
| `idle` | Data-bound | N/A | The default/rest data-bound state; rarely declared explicitly since it's what a node shows absent any other `state:` |

## 2. Why `press` (and `change`) can have two independent consequences

A `button` with `on:press=mcp:video_player.play` and the `Pressable` mixin applied does **two unrelated things** the instant it's pressed, and it is important that they are unrelated:

1. **The `Pressable` mixin's `on:press => scale=0.97 motion=snappy` transition fires immediately, inside the UI Runtime, with zero dependency on anything else.** This is the motion event Principle 4 is about. It happens whether or not the node has any `mcp:` binding at all, and whether or not the eventual MCP call succeeds, fails, or is still pending.
2. **Separately, the `on:press=mcp:video_player.play` binding fires an MCP call.** This is the intent event. It may resolve instantly (a registered lambda handles it) or take longer (it wakes the Agent Core, per §3 of `04-events/03-intent-routing.md`) — and *that* latency is never allowed to delay consequence 1.

These are declared in two different places (the `Pressable` **mixin**, in ASL, vs. the node's own `on:press=` **prop**, in AUIL) that happen to share the trigger name "press." A node can have either without the other: a `Card` has `Hoverable`'s motion response but typically no `on:press=` binding at all (`03-widgets-and-types/03-states-and-variants.md` §4); a `list` item might have an `on:press=mcp:` binding with no local motion mixin if it's meant to feel instantaneous rather than tactile. `change` follows the identical pattern — the value commits to its `@`-bound State Store path unconditionally (this is not optional and not itself a motion event, since it's a deterministic but non-instantaneous write, not a visual transition), and *additionally* fires an `on:change=mcp:` intent if one is declared.

## 3. Payload shape

An input-triggered event carries, at minimum, the target node's `id`, the event name, and a timestamp; `press`/`release`/`drag` additionally carry pointer/keyboard-origin information (which the UI Runtime uses, not which an intent handler typically needs); `change` carries the new committed value. A data-bound (`state:`) transition carries no independent payload of its own — it's a rendering response to a value that already exists at the bound path, not an event with content in its own right.

## 4. What this document does not define

- **Intent resolution** — how an `mcp:target` string becomes a running handler, and what happens when none exists yet — is `04-events/03-intent-routing.md`.
- **Which physical inputs produce which of the events above** (a spoken utterance producing a `change` on a voice-mode `field`, a keyboard `Enter` producing the same `press` a pointer click would) is `04-events/02-input-and-interaction.md`.
- **Live-region announcement rules** for patches arriving as a result of any of this are `01-hig/02-accessibility.md` §8, not repeated here.

---

*Cross-references: `01-hig/01-design-principles.md` (Principle 4), `01-hig/02-accessibility.md` §6, §8 (keyboard activation parity, live regions), `02-style/06-motion.md` (the curves `on:`/`state:` transitions reference), `03-widgets-and-types/03-states-and-variants.md` (the state model these events drive), `04-events/02-input-and-interaction.md`, `04-events/03-intent-routing.md`.*
