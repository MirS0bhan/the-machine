# States and Variants

Principle 6 (`01-hig/01-design-principles.md`) says a component's appearance is a pure function of its mixins, its interaction state, and its `@`-bound values. This document is the state machine that makes that claim checkable, and the variant axes that are the other input to "appearance" besides state.

## 1. The two kinds of state, restated precisely

`design-system/README.md`'s notation guide already draws this line; here it's the primary subject rather than a side note.

| Kind | Declared | Driven by | Examples |
|---|---|---|---|
| **Input-triggered** | `on:event => props [motion=name]` | A raw pointer/keyboard event, resolved entirely inside the UI Runtime | `hover`, `press`, `release`, `focus`, `blur`, `drag` |
| **Data-bound** | `state:name → props [motion=name]` | A variant flag or a State Store value changing, independent of any input event | `loading`, `error`, `selected`, `disabled`, `empty` |

This resolves `ui-engine/README.md` open item 1 ("ASL `state:` prefix"): `state:` transitions are for exactly the states that are not raw input events — a component can be `loading` with nobody's finger anywhere near it.

## 2. Core interaction states

Every interactive primitive participates in this set unless explicitly noted:

| State | Trigger | Visual default | Mixin that supplies it |
|---|---|---|---|
| `idle` | (absence of any other state) | The mixin's base properties | — |
| `hover` | Pointer enters the hit area | `scale=1.02`, `elev` steps up one tier if the node has a `Surface` mixin, `motion=snappy` | `Hoverable` |
| `press` | Pointer/keyboard activation begins | `scale=0.97`, `motion=snappy` | `Pressable` |
| `focus` | Keyboard focus lands on the node | `border.focus` ring at 2px, offset from the edge | `Focusable` |
| `disabled` | `disabled=true` prop | Whole node at `opacity.disabled`; all `on:`/`state:` transitions except `disabled` itself are suppressed | `Disableable` |
| `selected` | `state:selected` — a data-bound variant (current nav item, checked radio, active tab) | `accent.subtle` background or `accent.default` border, per component; icon variant flips `outline`→`filled` (`02-style/04-iconography.md` §2) | `Selectable` |
| `loading` | `state:loading` — a data-bound variant tied to an async task | Content replaced or overlaid by an indeterminate `slider`/`Spinner`; interaction suppressed but node is not `opacity.disabled`-dimmed (loading is not the same statement as disabled) | `Loading` |
| `error` | `state:error` — a data-bound variant, typically on `field` after validation | `border.default` swaps to `status.destructive`; paired error text appears per `05-ui-ux-patterns/03-forms-and-data-entry.md` §3 | (applied directly by the `field`/`Field` definition, not a shared mixin — error styling is specific enough per-primitive that a shared mixin would need overrides more often than it would help) |

## 3. Precedence when multiple states are true at once

States are not mutually exclusive at the data level (a `selected` row can also be `hover`ed), so rendering needs a fixed precedence rather than "last mixin wins" for the cases where two states would otherwise visually fight:

1. **`disabled` overrides everything else.** A disabled-and-hovered button shows only the disabled treatment — hover/press/focus transitions are suppressed entirely while `disabled=true`, not merely visually dominated.
2. **`loading` overrides `hover`/`press` but not `selected`.** A loading tab can still show as the selected tab (so the person doesn't lose track of which tab they're waiting on) but won't show a press-squish if tapped again mid-load.
3. **`error` overrides `selected`'s border treatment on the same edge**, but not `selected`'s other visual signals (an errored, selected list row keeps its selected background tint and gains the destructive border).
4. **`hover`, `press`, and `focus` compose rather than override each other** — a focused button that's also being hovered shows both the focus ring and the hover elevation simultaneously; they don't share a visual channel, so there's no conflict to resolve.

## 4. Mixin-to-state map

| Mixin | Supplies | Applies to (examples) |
|---|---|---|
| `Surface` | Base fill, `elev`, `radius` — no state transitions of its own | `Card`, `Field`, `MediaPlayer`, `Chart` |
| `Hoverable` | `hover` | `Card`, `ListRow`, `PrimaryButton`, `IconBtn` |
| `Pressable` | `press` | `ListRow`, `PrimaryButton`, `IconBtn` (not `Card` — see below) |
| `Focusable` | `focus` | Every interactive primitive's default component wrapper |
| `Disableable` | `disabled` | Every interactive primitive |
| `Selectable` | `selected` | `ListRow` (as a nav/list item), tab items, radio/checkbox rows |
| `Loading` | `loading` | `PrimaryButton`, `Card` (when it represents an async task), `MediaPlayer` |
| `Divider` | (no state — a static hairline rule) | Bare `stack.Divider` |

**Why `Card` has `Hoverable` but not `Pressable`:** a card frequently contains its own independently-pressable children (a `button`, a link-like row) rather than being a single monolithic press target. Giving `Card` a press-squish of its own would visually compete with — and misrepresent — presses actually landing on something inside it. A card that genuinely *is* a single press target should compose `Card` with `Pressable` explicitly at the point of use, not gain it by default.

## 5. Variant axes (orthogonal to state)

Variants are authored choices, not runtime-triggered — they don't change without an explicit prop change, unlike states above.

| Axis | Values | Notes |
|---|---|---|
| **Size** | `sm` / `md` (default) / `lg` | Scales padding (`space` tiers) and, for text-bearing controls, the paired `text` `size=` step together — never adjusted independently, so a control's internal rhythm stays proportional |
| **Emphasis / tone** | `primary` / `secondary` / `tertiary` / `destructive` / `ghost` | Primary use case: `button`'s `variant=`. `primary` = solid `accent` fill; `secondary` = `border.default` outline, no fill; `tertiary` = no border or fill, text-only; `destructive` = solid `status.destructive` fill; `ghost` = no visual weight until hovered — reserved for the least-important action in a group |
| **Density** | `comfortable` (default) / `compact` | `02-style/07-layout-and-spacing.md` §3 — a surface-level setting, not usually set per-component |

A component MUST NOT invent a new value for any axis above ad hoc (a sixth button `variant=`, a `xl` size) without going through `07-governance/01-contribution-and-review.md` — variant axes are exactly as closed as the primitive set itself, for the same reason.

---

*Cross-references: `01-hig/01-design-principles.md` (Principle 6), `02-style/06-motion.md` (the curves referenced by `on:`/`state:` transitions here), `03-widgets-and-types/01-primitive-types.md` (per-primitive state support), `03-widgets-and-types/02-component-library.md` (which components combine which mixins), `05-ui-ux-patterns/03-forms-and-data-entry.md` §3 (the `error` state's field-specific treatment).*
