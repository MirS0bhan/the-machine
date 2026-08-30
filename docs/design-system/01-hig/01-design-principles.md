# Design Principles

These principles apply to every screen the agent composes, every component defined in `03-widgets-and-types/`, and every pattern in `05-ui-ux-patterns/`. Where a later document's guidance seems to conflict with a principle here, the principle wins — the later document has a bug, not the other way around.

## 1. Deference to intent

The interface exists to reflect what the person is trying to do, not to assert its own presence. Chrome — borders, dividers, decorative containers — should be the last thing added, not the first. A screen with too much visual structure and too little content is a defect, not a design choice.

**In practice:** default to `elev=e0` (flat) and no visible container unless a boundary genuinely needs to be communicated (a distinct card of unrelated content, a modal that must read as interruptive). Every `stack.Surface` added to a layout should be able to answer "what would be ambiguous without this container" — if the answer is nothing, remove it.

## 2. Depth signals hierarchy, not decoration

Elevation (`02-style/05-materials-and-elevation.md`), vibrancy, and shadow exist to answer one question for the eye: *what is above what, and what needs my attention first.* They are not applied for visual richness on their own. A raised surface without a reason to be raised — nothing floats above or beneath it, nothing about it demands more attention than its siblings — should be flattened.

**In practice:** elevation is not decoration budget to spend, it's a hierarchy signal with a fixed vocabulary (flat/raised/floating/overlay/system, `02-style/01-design-tokens.md` §scale `elev`). If two things are at the same importance level, they get the same elevation tier, full stop — mismatched elevation between visual peers is a bug a reviewer should flag the same way a broken layout would be flagged.

## 3. Consistency lives in tokens, not memory

No component, human designer, or agent turn should ever need to recall "what shade of gray did we use for secondary text last time." That answer is `text.secondary` — a token, resolved once, applied everywhere. This is a structural guarantee, not a style preference: raw hex values, raw pixel spacing, and raw duration numbers are not permitted as direct property values anywhere in AUIL or ASL source — they may only ever appear on the right-hand side of a `token` / `scale` / `motion` *declaration* itself (`ui-engine/docs/spec.md`'s grammar already establishes this at the parser level; this principle is why the rule exists).

**Why this matters more here than in typical design systems:** the primary author of most screens in this OS is an LLM composing UI turn by turn, often without the previous turn's reasoning in context. A token system isn't just tidiness — it's what makes correctness possible without memory. An agent that always reaches for `surface.card` produces a consistent app; an agent that occasionally free-hands a color because it seemed reasonable in the moment produces visual drift no reviewer will catch turn-by-turn.

## 4. Real-time locality

Nothing that must feel instantaneous — a hover highlight, a press squish, a focus ring, a drag — may wait on anything beyond the UI Runtime itself. This is inherited directly from the parent architecture's Design Commitment #2 (`docs/spec.md` §1 / `ARCHITECTURE.md` §1: "real-time paths never touch inference") and is why ASL's event model (`04-events/01-event-model.md`) draws a hard, syntactically visible line between motion events (`on:event =>`, resolved locally) and intent events (`mcp:` sigil, may cross the bus). A component that makes its hover state depend on an MCP round-trip is not a slow implementation of a correct design — it is an incorrect design.

## 5. Accessible by construction, not by review

Every primitive in `03-widgets-and-types/01-primitive-types.md` carries mandatory accessibility fields with sane defaults derived structurally from its content (a `button`'s label defaults from its text child; a `field`'s role is implied by its type). This means an agent that never thinks about accessibility at all still produces an accessible baseline, and an accessibility audit is checking for *overrides that made things worse*, not scanning for omissions in code that never had the fields to begin with.

**In practice:** if you find yourself writing a bespoke "add an accessible label" step as a separate task from writing the component, something upstream is wrong — the component definition should have made that unnecessary.

## 6. One state of truth per visual property

A component's rendered appearance at any moment is a pure function of: its ASL mixins, its current interaction state (idle/hover/press/focus/disabled — `03-widgets-and-types/03-states-and-variants.md`), and any `@`-bound State Store values. There is no hidden local styling state that isn't reconstructable from those three inputs. This is what makes the patch protocol (`~ + - ! @`) safe — a patch can update one prop without the runtime needing to guess what else might have silently diverged.

## 7. Native means predictable, not skeuomorphic

"Looks native" in this system does not mean mimicking any particular existing desktop's specific visual signature. It means: consistent elevation logic, a coherent type scale, spring-based motion that feels physically continuous rather than abruptly cut, and status communicated redundantly (icon + text + color, never color alone — principle 5's accessibility stance applied to color specifically). A screen that follows these rules will feel calm and predictable regardless of the specific hex values or corner radii chosen; a screen that violates them will feel foreign no matter how closely it copies a specific existing look.

## 8. The agent's job is composition, not invention

An agent producing a screen should almost always be *assembling* named tokens, mixins, and components (`03-widgets-and-types/02-component-library.md`), not inventing new visual treatments turn-by-turn. Novel visual treatments are expensive in the same way novel lambda functions are expensive (`lambda-server/docs/spec.md` §8) — they should be rare, deliberate, and (once proven useful) registered as a new named component rather than repeated as a one-off every time a similar need arises.

---

*These eight principles are referenced by number (e.g. "Principle 4") throughout the rest of this document set.*

*Cross-references: `02-style/01-design-tokens.md` (the `elev` scale and the token/scale/motion-only rule), `03-widgets-and-types/01-primitive-types.md` and `03-widgets-and-types/03-states-and-variants.md` (accessible-by-construction defaults and the state model), `04-events/01-event-model.md` (motion vs. intent events), `05-ui-ux-patterns/02-feedback-and-status.md` (redundant status coding).*
