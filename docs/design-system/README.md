# The Machine Design System — Visual Language Documentation

**Fills:** §3.6.2 of `docs/spec.md` (Declarative UI Runtime design requirements), the accessibility/theming/layout open items in `ui-engine/README.md`, and the "exact declarative UI schema" item in `ARCHITECTURE.md` §7
**Related:** `ui-engine/docs/spec.md` (the AUIL/ASL grammar this document set assumes and does not redefine), `docs/components/ui-runtime.md`, `docs/components/compositor.md`, `docs/components/policy-broker.md` (the Confirmation Surface mechanics that `05-ui-ux-patterns/04-agent-presence-and-conversation.md` gives a visual design to), `docs/architecture/philosophy.md`
**Version:** 0.1
**Status:** Design reference — normative for new UI work in The Machine

---

This is the complete design-system documentation for The Machine's visual language: the structure language (**AUIL**, Agent UI Layout) and the style-and-motion language (**ASL**, Agent Style Language) implemented in `ui-engine/` and specified in `ui-engine/docs/spec.md`, expanded here into full guidelines, tokens, a widget catalog, an event model, and interaction patterns.

This documentation set is self-contained and normative. Where it defines a value, a rule, or a component, that definition is the canonical one for implementation — it does not merely illustrate the AUIL/ASL grammar, it completes it. Nothing here is aspirational flavor text: every MUST is something the UI Runtime, the Policy Broker, or a code reviewer can actually check.

## Relationship to the implemented AUIL/ASL grammar

Everything in this document set uses the grammar already implemented in `ui-engine/` and specified in `ui-engine/docs/spec.md` exactly as it exists today: the eleven primitive tags, the five patch operations, the three reference sigils, and the `token` / `scale` / `motion` / `style` declaration forms. Nothing here changes that grammar, and nothing here requires a parser change to express.

Four things in this document set go beyond what `ui-engine/components.py` currently registers or `ui-engine/README.md` currently leaves open, and each is called out once, at its point of definition, rather than hidden or restated everywhere it's used:

- **`dialog` is specified as a twelfth fixed primitive** (`03-widgets-and-types/01-primitive-types.md` §12). Modal focus-trapping and exclusive input routing are behavioral guarantees that don't reduce to a styling composition of the other eleven primitives, the same way `media`'s stream binding doesn't reduce to a bare `stack`.
- **The full token, scale, and motion value catalogs** (`02-style/`) are new. The implemented grammar defines *how* a token, a scale, or a motion curve is declared; this document set is where the complete, canonical set of *values* — every color, every radius, every duration — is defined for the first time. The few token/scale examples already present in `ui-engine/docs/spec.md`, `ui-engine/README.md`, and `ui-engine/test_engine.py` (`surface.primary`, `accent`, `scale radius: sm=6 md=10 lg=16`, `scale space: xs=4 sm=8 md=12 lg=16 xl=24`, `motion snappy`, `motion gentle`) are parser fixtures, not a finished palette — this document set keeps their exact names and numbers and completes the rest of the system around them.
- **The component library** (`03-widgets-and-types/02-component-library.md`) extends the eight components currently registered in `ui-engine/components.py` (`Surface`, `Card`, `ListRow`, `PrimaryButton`, `IconBtn`, `Field`, `MediaPlayer`, `Chart`) with the rest of the catalog a full desktop needs. All eight keep their exact existing name, parent, and mixin composition.
- **This document set resolves all seven open items listed in `ui-engine/README.md`** ("ASL `state:` prefix," slot defaults, token inheritance, style attachments, layout algorithm contract, accessibility mapping, compositor-level theming broadcast) and the local/cloud-adjacent open item in `docs/agent-core-spec.md` §12 about protected-unit confirmation UX. Each resolution is called out by name at its point of definition instead of collected into one list, so it reads as settled design rather than a running commentary on old open items.

If you are looking for how AUIL is parsed, how a patch is transmitted over MCP, or how the compositor binds a Wayland surface — that is `ui-engine/docs/spec.md`, `docs/components/ui-runtime.md`, and `docs/components/compositor.md`. This folder never repeats that material; it only cites it.

## How this fits together

```
docs/spec.md                      — system architecture, why an agent mediates the OS
ui-engine/docs/spec.md            — the AUIL/ASL grammar (parsing, patch protocol, sigils)
docs/agent-core-spec.md           — the harness that emits AUIL/ASL at runtime
lambda-server/docs/spec.md        — how UI-bound intents get a deterministic handler

design-system/                    — THIS DOCUMENT SET: what the language should say,
                                     not how it's parsed. Principles, tokens, components,
                                     events, and patterns that a designer or an agent
                                     needs in order to produce a correct, native-feeling
                                     interface using the grammar the other docs define.
```

If `ui-engine/docs/spec.md` is the grammar of a language, this folder is its dictionary, style guide, and idiom book.

## Folder map

| Folder | Contents |
|---|---|
| `01-hig/` | Design principles, accessibility standards, content and voice guidelines, and inclusive/adaptive-design rules — the non-visual rules that shape every decision below |
| `02-style/` | Design tokens, color and surfaces, typography, iconography, materials and elevation, motion, and layout/spacing — the complete visual vocabulary |
| `03-widgets-and-types/` | The fixed primitive type set, the composed component library, the state/variant model, and composition/responsive-layout rules |
| `04-events/` | The event model, input and interaction handling, intent routing, and multimodal/voice input — how the UI talks to the rest of the system |
| `05-ui-ux-patterns/` | Navigation and layout, feedback and status, forms and data entry, agent presence and conversation, empty/loading/error states, and multitasking/surfaces — how the primitives and components combine into actual screens |
| `06-glossary.md` | Every term used across this document set, defined once |
| `07-governance/` | How a token, mixin, component, or primitive is proposed, reviewed, versioned, and deprecated |

## Reading order

- **Implementing a new primitive or the parser itself:** start with `02-style/01-design-tokens.md`, then `03-widgets-and-types/01-primitive-types.md`.
- **Composing a screen (agent or human designer):** start with `01-hig/01-design-principles.md`, then whichever pattern in `05-ui-ux-patterns/` matches the task, pulling components from `03-widgets-and-types/02-component-library.md` as needed.
- **Wiring a new intent or handler:** `04-events/01-event-model.md` then `04-events/03-intent-routing.md`.
- **Auditing for accessibility or content quality:** `01-hig/02-accessibility.md` and `01-hig/03-content-and-voice.md`.
- **Designing for a screen size, locale, or input modality you haven't touched before:** `01-hig/04-inclusive-and-adaptive-design.md`, then `03-widgets-and-types/04-composition-and-responsive-layout.md`.
- **Proposing a new token, mixin, or component:** `07-governance/01-contribution-and-review.md`.

## Notation used throughout this document set

This document set cites the real AUIL/ASL grammar constantly. The notation below is not new grammar — it is exactly what `ui-engine/docs/spec.md` and `ui-engine/README.md` already implement — collected here once so the rest of the folder can use it without re-explaining it every time.

**AUIL node grammar** (structure): a line is `tag[.Mixin1.Mixin2][#id][(prop=val ...)] ["text content"]`, indentation nests children two spaces per level. Casing carries meaning: a **lowercase** tag (`stack`, `button`, `field`) is one of the fixed primitives (`03-widgets-and-types/01-primitive-types.md`); a **PascalCase** name in the mixin-chain position or used as the tag itself is a mixin or a registered component (`03-widgets-and-types/02-component-library.md`). For example, `stack.Surface.Hoverable#card(gap=md)` is a `stack` primitive with the `Surface` and `Hoverable` mixins applied, addressed as `#card`.

**Reference sigils** (property values that point outside the literal): `$lambda:path` binds to a running lambda's output (a media stream, a computed value); `mcp:method` names an MCP intent a `press`/`change`/etc. should invoke; `@path` binds to a State Store path, two-way where the primitive supports it (e.g. `value=@player.position`). A bare, unprefixed value is always a literal — a token, a scale tier, a motion name, a number, or a string — never a sigil.

**Token references** (`02-style/01-design-tokens.md`): tokens are declared `token category.role = value` (e.g. `token surface.card = adaptive(light:#FCFCFF dark:#15161F)`) and referenced inline with a `token:` prefix (`bg=token:surface.card`). Token names are always written `category.role` in prose — `surface.card`, `text.secondary` — even where the inline reference needs the `token:` prefix.

**Scale references** (`02-style/01-design-tokens.md`): a scale is declared `scale name: tier=value tier=value ...` (e.g. `scale radius: sm=6 md=10 lg=16`). Radius and space scales are referenced inline with a one-letter shorthand prefix — `r-lg`, `s-md` — because their tier names (`sm`, `md`, `lg`) would otherwise read as ambiguous bare words. The elevation scale is the one exception: its tiers (`e0`–`e4`) are self-describing and referenced bare (`elev=e2`), never with a prefix.

**Motion references** (`02-style/06-motion.md`): a motion curve is declared `motion name = spring(stiffness=N damping=N)` or `motion name = duration(Nms ease=name)`, and referenced by bare name (`motion=snappy`). Do not confuse this with the unrelated `scale=` **property** (a numeric transform multiplier around `1.0`, e.g. `scale=1.02` for a hover grow) — the word "scale" names both a token-ramp declaration keyword and a visual-transform property, and the two never appear in the same position in a line.

**State transitions inside a `style` block** (`03-widgets-and-types/03-states-and-variants.md`, `04-events/01-event-model.md`): two forms, and they answer different questions.
- `on:event => props [motion=name]` — an **input**-triggered, purely local transition (`hover`, `press`, `release`, `focus`, `blur`, `drag`). Never crosses the MCP bus. This is what Principle 4 (`01-hig/01-design-principles.md`) means by a motion event.
- `state:name → props [motion=name]` — a **data**-bound transition driven by a variant or a State Store value (`loading`, `error`, `selected`, `disabled`, `empty`). Resolves whenever the bound value changes, independent of any pointer or keyboard input.

**Patch operators** (`ui-engine/docs/spec.md`, cited here, not redefined): `~id(props)` update, `+anchor: node` insert, `-id` remove, `!id: node` replace, `@id → other-id` move. This document set only ever describes what a patch *should* target and *why*, never how the patch protocol itself works.

**Normative language:** "MUST" / "SHOULD" / "MAY" are used in the RFC sense — MUST is a hard requirement enforceable by the Policy Broker or the UI Runtime, SHOULD is a strong default an implementation needs a real reason to deviate from, MAY is a genuine option.

**Cross-references:** every file in this folder ends with a short italicized cross-reference line naming the other files a reviewer would need to check a claim against. Follow them — this document set is written so that no single file needs to restate a rule owned by another.

*End of index.*
