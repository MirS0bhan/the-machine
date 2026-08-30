# Motion

Motion in ASL is declared with exactly two forms, introduced in `design-system/README.md`'s notation guide and given their full treatment here: `on:event => props [motion=name]` for input-triggered local transitions, and `state:name → props [motion=name]` for data-bound transitions. This document defines the named curves both forms reference, and the choreography rules that keep motion feeling like one coherent system rather than a per-component improvisation.

## 1. Named curves

| Name | Form | Parameters | Feel | Default use |
|---|---|---|---|---|
| `snappy` | `spring` | stiffness=300, damping=26 | Quick settle, minimal overshoot | Hover/press micro-feedback (`Hoverable`/`Pressable` mixins), icon state cross-fades |
| `gentle` | `duration` | 300ms, ease-out | Smooth, deliberate | Content entering/settling — a card's `Surface` mixin resting after insertion, agent-initiated (not user-caused) UI arrivals per `01-hig/04-inclusive-and-adaptive-design.md` §6 |
| `standard` | `spring` | stiffness=260, damping=24 | General-purpose, barely more travel than `snappy` | Default for any transition that isn't specifically hover/press or a large-surface entrance |
| `emphasized` | `spring` | stiffness=180, damping=20 | Slower, more travel, a touch of overshoot | High-attention transitions — a `dialog` entering at `e3`, a surface-level navigation change |
| `exit` | `duration` | 160ms, ease-in | Quick, no overshoot | Dismissals — a `Toast` leaving, a `dialog` closing |
| `reduced` | `duration` | 120ms, linear | Instant-feeling crossfade | The mandatory substitution for every curve above when reduced motion is active (`01-hig/02-accessibility.md` §5) — never selected explicitly by a component |

`snappy` and `gentle` are also the two names already fixed by `ui-engine/test_engine.py`'s parser fixtures and `ui-engine/README.md`'s example — this table keeps both exactly as they are and completes the rest of the set around them.

## 2. Choreography rules

1. **One curve per transition, never blended.** A single prop change references exactly one `motion=` name; don't stagger multiple curves on the same property of the same node.
2. **Direction of motion must be causally consistent.** A dismissed `Toast` MUST exit toward the edge it entered from (or the edge it's anchored to), never a different direction — motion that doesn't share a spatial logic with its own entrance reads as arbitrary rather than physical.
3. **Entrance and exit are asymmetric on purpose.** Entrances use the calmer end of the curve set (`gentle`, `standard`, `emphasized`); exits use `exit`, which is deliberately quicker and overshoot-free — a thing leaving the screen doesn't need to draw attention the way a thing arriving does.
4. **Lists stagger, individual props don't.** When a `list` inserts multiple new rows at once (a batch `+` patch), successive rows MAY stagger their `gentle` entrance by a small, fixed per-row delay (≈24ms) so the eye can track them arriving as a group; a single row's own internal properties (its background, its text) never stagger relative to each other.
5. **Agent-initiated motion stays calm.** Per `01-hig/04-inclusive-and-adaptive-design.md` §6, a patch the person did not directly cause (a background task finishing, a proactive suggestion appearing) uses `gentle`, never `snappy` or `emphasized`, regardless of what a user-caused equivalent in the same spot would use.
6. **Motion never substitutes for state.** A value MUST reach its final, correct state at the end of a transition's declared duration even if the runtime is under load and drops frames — motion is a perceptual aid, never the mechanism by which a value becomes correct (this is what keeps Principle 6, "one state of truth," true even mid-animation).

## 3. Reduced motion substitution

Per `01-hig/02-accessibility.md` §5, every `motion=` reference resolves to `motion.reduced` automatically when the system reduced-motion preference is active — a component author never writes a reduced-motion branch. What changes is *how* the end state is reached (an instant-feeling 120ms linear crossfade instead of a spring), never *whether* it's reached, and never which properties change — a `Hoverable` mixin still communicates hover under reduced motion, just without the `scale=1.02` physicality.

## 4. What has no motion token, deliberately

Per `01-hig/04-inclusive-and-adaptive-design.md` §6, parallax and any scroll-position-driven or pointer-position-driven differential motion across layers has no token in this system and MUST NOT be implemented by reaching for a combination of existing curves to approximate it — the absence is the rule, not a gap to fill.

---

*Cross-references: `01-hig/01-design-principles.md` (Principle 4, real-time locality), `01-hig/02-accessibility.md` §5 (reduced motion), `01-hig/04-inclusive-and-adaptive-design.md` §6 (vestibular sensitivity, agent-initiated motion), `04-events/01-event-model.md` (the `on:` vs `state:` transition forms these curves attach to), `03-widgets-and-types/03-states-and-variants.md` (which mixins reference which curves by default).*
