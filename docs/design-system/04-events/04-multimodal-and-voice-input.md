# Multimodal and Voice Input

Voice is a first-class input, not an accessibility add-on bolted onto a pointer-and-keyboard system — the login greeting itself pairs a spoken affordance with a typed one from the first screen a person sees (`05-ui-ux-patterns/04-agent-presence-and-conversation.md` §1). This document defines the visual states voice interaction needs on top of the event vocabulary in `04-events/01-event-model.md`, which voice does not extend, only feeds into (`04-events/02-input-and-interaction.md` §3).

## 1. `input-mode=` on `field`

| `input-mode=` | Rendered affordance |
|---|---|
| `text` | An ordinary text-entry `field` |
| `voice` | A microphone-forward `field` — a mic `icon` is the primary affordance, with typed entry still available as a fallback, not removed |
| `hybrid` | Both are equally primary — a text-entry `field` with a persistent mic `icon` adornment (the pattern used by `SessionGreeting`, `03-widgets-and-types/02-component-library.md` §9) |

Whichever mode is set, a resolved utterance or a resolved typed string produces the identical `change` event (`04-events/01-event-model.md`) — voice does not have its own event type, only its own path to producing the same one.

## 2. Voice presence states

A voice-capable `field` (or a dedicated ambient voice affordance, where one exists outside any single `field`) cycles through a small, fixed set of `state:`-bound visual states — data-bound, not input-triggered, per `04-events/01-event-model.md`, because these reflect the state of an ongoing process (listening, transcribing) rather than a single discrete input event:

| State | Visual signal | Announce (`01-hig/02-accessibility.md` §8) |
|---|---|---|
| `idle` | Mic `icon`, `outline` variant, `text.secondary` color | — |
| `listening` | Mic `icon` switches to `filled` variant, `accent.default` color, a subtle `Spinning`-adjacent pulse (not a full spin — a gentle scale pulse using `motion.gentle`, looping) | `polite`, once, on entering this state |
| `transcribing` | The `field`'s live (uncommitted) transcript renders in `text.secondary` inside the input area as it arrives, distinct from committed `text.primary` value text | Silent — this is a routine, continuously-updating content patch (`01-hig/02-accessibility.md` §8), not new information until committed |
| `processing` | Mic `icon` returns to `outline`; the field (or the surface it belongs to) may separately show the processing-locus indicator (§4 below) if the resulting intent escalates | — |

## 3. Barge-in

A person MUST be able to start a new utterance while the system is still delivering a spoken response — this is a capability of the underlying audio pipeline, not of AUIL/ASL, but the **visual** consequence is in scope here: a barge-in interrupts whatever visual state (e.g. a `state:speaking`-equivalent indicator, if the surface has one) was showing and transitions immediately to `listening`, with no confirmation step and no delay. Nothing about barge-in is allowed to depend on an MCP round-trip — it's a real-time, Principle-4 interaction the same as a `press`.

## 4. Voice/text parity

Every action reachable by voice MUST have a visual, text-legible equivalent, and every visually-triggered action's outcome MUST be representable in whatever the voice channel narrates back — this is `01-hig/02-accessibility.md` §6's keyboard/pointer parity requirement, extended to voice as a third equally-first-class modality rather than treated as a separate accessibility feature. Concretely: a `SuggestionChip` the agent offers proactively (`05-ui-ux-patterns/04-agent-presence-and-conversation.md` §4) MUST be speakable/nameable by voice with the same result its `on:press` binding would produce visually, and an agent narrating a status update out loud MUST be saying something that also exists as `text` content on screen, not exclusively as speech.

## 5. Voice does not bypass confirmation

A voice-initiated request that would otherwise require `CONFIRM` (a protected-unit action, a sensitive capability grant, per `docs/components/policy-broker.md` § Protected Units) goes through the identical Confirmation Surface a pointer-initiated request would — there is no voice-specific approval path, and specifically no "confirm by saying yes" shortcut that bypasses the Confirmation Surface's provenance-checked, unforgeable input path (`docs/components/policy-broker.md` § Input Provenance). Voice can *ask* for anything; it cannot *approve* the one class of thing this system reserves for a verified physical, provenance-marked input event.

## 6. Extensibility

Gesture and eye-tracking, if ever added (`ARCHITECTURE.md` §7 item 5), follow the same pattern this document establishes for voice: a new way of producing the existing `04-events/01-event-model.md` event vocabulary, with its own presence-state visuals where the modality needs them (as voice needed §2's states), rather than a new event type or a parallel component library.

---

*Cross-references: `01-hig/02-accessibility.md` §6, §8 (parity and announcement rules this document extends to voice), `04-events/01-event-model.md` (the `change` event voice ultimately produces), `04-events/02-input-and-interaction.md` §3 (voice's place in the input map), `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §1, §4–§5 (the greeting flow, suggestion chips, and processing-locus indicator that voice interacts with).*
