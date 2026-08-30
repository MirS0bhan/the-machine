# Contribution and Review

The vocabulary this document set defines — tokens, scales, motion curves, mixins, primitives, components — is shared state every screen depends on (`01-hig/01-design-principles.md` Principle 3). Changing it is different from changing one screen, and goes through a different, explicit process rather than accreting through individual component authors' local judgment calls.

## 1. What requires review under this process

| Change | Review bar |
|---|---|
| A new value on an existing token/scale (a new `radius` tier, a new `status` hue) | Must show the existing set genuinely can't express the need — not merely that a slightly different number would look nicer in one spot |
| A new token in an existing category | Must be defined with both light and dark values (or an explicit derivation, `02-style/01-design-tokens.md` §4) and pass `01-hig/02-accessibility.md` §3's contrast requirement against every surface it's meant to pair with |
| A new mixin | Must map to a state or base treatment not already covered by the existing set in `03-widgets-and-types/02-component-library.md` §0 |
| A new component | Must show the composition it names has repeated at least three times across independently-motivated screens (`03-widgets-and-types/04-composition-and-responsive-layout.md` §4) — a component proposed after one use is speculative, not observed |
| A new primitive | The highest bar in this document set: must show the need is a *behavioral* guarantee (like `dialog`'s focus trap) that cannot be expressed as any composition of the existing twelve plus mixins — most proposals at this level should end up as a component instead |
| A new variant axis or value (`03-widgets-and-types/03-states-and-variants.md` §5) | Same bar as a new mixin — must cover a real, recurring need, not a one-off |

A change that only affects how a single, specific screen looks — without touching a shared name — needs no review under this document; it's just using the system.

## 2. Proposal contents

A proposal for anything in §1 states, at minimum:

1. **The specific gap** — what existing vocabulary was reached for and found insufficient, with real examples, not a hypothetical.
2. **The exact addition** — the token/scale tier/mixin/component/primitive definition, written in the same form this document set already uses (a table row, following the existing pattern for that category) — a proposal that isn't already in the shape the relevant file expects isn't ready for review.
3. **Every file the addition touches** — including the glossary (`06-glossary.md`) and any cross-reference lines in files that will now need to point at it.
4. **For anything touching accessibility defaults, contrast, or the Confirmation Surface specifically:** an explicit statement of which `01-hig/02-accessibility.md` requirement or which `docs/components/policy-broker.md` mechanic the addition must remain compatible with.

## 3. Review checklist

- [ ] Does it use existing tokens/scales rather than introducing a raw literal anywhere it doesn't have to (Principle 3)?
- [ ] Does every new interactive element resolve a non-empty label, a fixed role, and a minimum hit target by construction (`01-hig/02-accessibility.md` §1–§2, §9)?
- [ ] Does every new color pairing meet the relevant contrast threshold, evaluated after vibrancy (`01-hig/02-accessibility.md` §3)?
- [ ] Does it degrade correctly under reduced motion, reduced transparency, and high contrast without a component-specific branch (`01-hig/02-accessibility.md` §4–§5)?
- [ ] Does it work under `compact` size class and under right-to-left layout without a special case (`01-hig/04-inclusive-and-adaptive-design.md` §2–§3)?
- [ ] Is its copy, if any, consistent with `01-hig/03-content-and-voice.md`'s terminology table and voice rules?
- [ ] Has the glossary and every affected file's cross-reference line been updated?

## 4. Who approves what

Ordinary tokens, mixins, and components are a design review — a human design owner (or, per Principle 8, an agent turn that has correctly identified a repeated pattern and is registering it, subject to the same checklist) signs off against §3's checklist. Anything that would change the Confirmation Surface's anatomy (`03-widgets-and-types/02-component-library.md` §8) or the `e4` elevation reservation is **not** a design-system-only decision — it changes what the Policy Broker's Confirmation Surface Daemon renders, and requires the same review the Broker's own template changes would (`docs/components/policy-broker.md` § Template System: "hand-authored, fixed... the agent cannot create or modify templates" applies to proposals as much as to runtime behavior).

---

*Cross-references: `01-hig/01-design-principles.md` Principle 3, 8, `01-hig/02-accessibility.md` (the checklist's structural requirements), `03-widgets-and-types/02-component-library.md` §8 (the one part of this catalog outside normal design-review authority), `07-governance/02-versioning-and-lifecycle.md` (how an approved change is released).*
