# Input and Interaction

This document maps physical input — pointer, keyboard, and (in overview; full treatment in `04-multimodal-and-voice-input.md`) voice — onto the fixed event vocabulary of `04-events/01-event-model.md`. The design commitment here is the one `01-hig/02-accessibility.md` §6 states directly: one semantic action, one intent name, regardless of which input modality produced it.

## 1. Pointer

| Physical action | Event(s) produced |
|---|---|
| Pointer enters a hit area | `hover` |
| Pointer leaves a hit area | `hover` clears (returns to `idle`) |
| Pointer down inside a hit area | `press` begins |
| Pointer up inside the same hit area it went down in | `press` completes — the bound `on:press` intent, if any, fires |
| Pointer up outside the hit area it went down in, or pointer otherwise leaves before release | `release` without a completed `press` — no intent fires |
| Pointer moves while held on a `draggable`-capable node | `drag`, continuously, until release |

Hit areas are never smaller than `space.min-target` (`01-hig/02-accessibility.md` §9) regardless of the primitive's visual size — the input layer and the rendering layer can disagree about size on purpose, and the input layer always wins toward the larger area.

## 2. Keyboard

- **Sequential navigation:** Tab/Shift-Tab move focus through interactive primitives in the order they appear in the current UI State Tree (`01-hig/02-accessibility.md` §6), except inside a component that declares roving focus (below) or a `dialog`'s trapped tab order (`01-hig/02-accessibility.md` §7).
- **Activation:** Enter/Space produce the identical `press` event a pointer click would, targeting the focused node — there is no separate "keyboard-activated" variant of `press` for an intent binding to distinguish, by design.
- **Roving focus within composites:** a `list` with `select=` set, a `Menu`, and a `TabBar`/`SegmentedControl` (`03-widgets-and-types/02-component-library.md` §2–§3) use arrow keys to move focus *among their own items* rather than Tab moving between every individual item — Tab moves focus into and out of the composite as a whole; arrow keys move within it. This is a property of the `list` primitive's `select=` mode, not something each composed component re-implements.
- **System-reserved shortcuts:** a small set of key combinations (e.g. the safe-mode combination that activates the Fallback Shell, `docs/components/fallback-shell.md`) are intercepted below the UI Runtime entirely and MUST NOT be rebindable or interceptable by any agent-authored patch — they exist specifically for the case where the UI Runtime itself is not to be trusted or is unavailable.

## 3. Voice (overview)

A `field(input-mode=voice)` or `input-mode=hybrid` produces the same `change` event (and, where bound, the same `on:change=mcp:` intent) a typed value commit would — an utterance that resolves to a value is functionally identical, from the intent-routing point of view, to typing that value and pressing Enter. The presence-and-transcription visual states (listening, transcribing, barge-in) that voice input adds on top of this are `04-events/04-multimodal-and-voice-input.md`'s subject, not this file's — they're additional *visual feedback*, not additional *event types*.

## 4. Extensibility

Per `01-hig/04-inclusive-and-adaptive-design.md` §4 and `ARCHITECTURE.md` §7 item 5, this system's input model does not assume pointer-only interaction today, specifically so that a future input family (touch, gesture) is an additional row in §1's table and a `space.min-target` recheck, not a redesign of the event vocabulary in `04-events/01-event-model.md`. A new input family MUST map onto the existing `hover`/`press`/`release`/`drag`/`change` vocabulary rather than introducing new event names — the vocabulary describes semantic outcomes, not input hardware, and that's what keeps one AUIL binding valid across every input modality that can produce it.

---

*Cross-references: `01-hig/02-accessibility.md` §6, §9 (keyboard parity, minimum target size), `01-hig/04-inclusive-and-adaptive-design.md` §4 (input-modality-agnostic design), `04-events/01-event-model.md` (the event vocabulary this file maps input onto), `04-events/04-multimodal-and-voice-input.md` (voice-specific visual states), `docs/components/fallback-shell.md` (the one system-reserved shortcut this document set is aware of).*
