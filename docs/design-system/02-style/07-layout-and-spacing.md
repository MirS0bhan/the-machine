# Layout and Spacing

This document is the practical application layer of the `space` scale (`02-style/01-design-tokens.md` §3): how much space, at what surface size, and how density and size class interact. `03-widgets-and-types/04-composition-and-responsive-layout.md` covers how individual primitives adapt; this file covers the shared grid and breakpoints every one of those adaptations is measured against.

## 1. Size classes

A `stack`/`grid`'s size class is resolved from the *available inline-axis space of the surface it renders into*, not from a device category — the same AUIL source produces the right layout whether that surface is a small panel or a maximized canvas, because the source never hardcodes a size class itself.

| Size class | Inline-axis space | Default margin (`space` scale) | Typical context |
|---|---|---|---|
| `compact` | < 480px | `s-lg` (16px) | A narrow panel, a split-view secondary pane, a small auxiliary surface |
| `standard` | 480–960px | `s-xxl` (32px) | A typical single-purpose surface at its default size |
| `expansive` | > 960px | `s-xxxl` (48px), capping content measure per `02-style/03-typography.md` §5 rather than letting margins grow unbounded | A maximized surface, multi-column content |

Size class is a property of the *surface*, re-evaluated on resize — it is not fixed at surface-creation time. A component that reads its own size class once and caches the decision will render incorrectly after the surface is resized; size class MUST be re-derived on every layout pass.

## 2. Grid

`grid` primitives (`03-widgets-and-types/01-primitive-types.md` §2) lay out on a column system rather than fixed pixel widths:

| Size class | Columns | Gutter |
|---|---|---|
| `compact` | 4 | `s-sm` (8px) |
| `standard` | 8 | `s-lg` (16px) |
| `expansive` | 12 | `s-xl` (24px) |

A `grid` child's `span=` prop is always relative to the active column count, never a literal pixel or fraction — this is what lets the same `grid` definition reflow correctly across size classes without an agent needing to author three versions of it.

## 3. Density

Two presets modulate the `space` scale's effective step size without changing which token names exist:

| Density | Effect | Where it applies |
|---|---|---|
| `comfortable` (default) | `space` scale used at its defined values | General-purpose surfaces |
| `compact` | Each `space` tier resolves one step down its own ramp (e.g. a `gap=s-lg` request resolves to the `md` value) | Information-dense contexts a person has explicitly opted into (a long settings list, a data table) — never the system default |

Per `01-hig/04-inclusive-and-adaptive-design.md` §4, density MUST NOT compress `space.min-target` — hit targets are a token, not a scale tier, specifically so density changes can never accidentally shrink them.

## 4. Safe margins and edge behavior

- Content MUST NOT render flush to a surface's physical edge except intentionally full-bleed media (`media` primitive) or a surface that is itself edge-to-edge by design (a status strip). Everything else respects the size class's default margin from §1.
- A vibrant or elevated surface's shadow/blur extent (`02-style/05-materials-and-elevation.md`) is not counted as content for margin purposes — margins are measured to the surface's layout box, not its visual bounding box including shadow spread.

## 5. Stack and grid sizing keywords

Every child of a `stack`/`grid` resolves its size along the container's axis to one of three keywords, never a bare pixel value that would fight the container's own responsiveness:

| Keyword | Behavior |
|---|---|
| `hug` | Sizes to its own content's natural size |
| `fill` | Grows to consume available space along the axis, sharing proportionally with sibling `fill` children |
| `fixed(value)` | A literal size from the `space` scale only (e.g. `fixed(s-huge)`) — never a raw pixel number, per Principle 3 |

---

*Cross-references: `01-hig/04-inclusive-and-adaptive-design.md` §3–§4 (size class and density rationale), `02-style/01-design-tokens.md` §3 (the `space` scale these margins/gutters draw from), `03-widgets-and-types/04-composition-and-responsive-layout.md` (per-component adaptation across these size classes), `03-widgets-and-types/01-primitive-types.md` §1–§2 (`stack`/`grid` primitives).*
