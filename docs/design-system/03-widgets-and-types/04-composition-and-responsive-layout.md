# Composition and Responsive Layout

`02-style/07-layout-and-spacing.md` defines the size classes and grid; this document defines how primitives and components actually behave as a surface moves between them, and the composition discipline that keeps deeply nested AUIL trees from becoming their own maintenance burden.

## 1. Alignment and distribution, applied

`stack`'s `align=` (cross-axis) and `distribute=` (main-axis) props, and `grid`'s equivalent, take exactly four and four values respectively (`03-widgets-and-types/01-primitive-types.md` §1) — no bespoke per-component alignment logic exists outside these.

| Prop | Values | Axis |
|---|---|---|
| `align=` | `start` \| `center` \| `end` \| `stretch` | Cross-axis (perpendicular to `dir=`) |
| `distribute=` | `start` \| `center` \| `end` \| `between` | Main-axis (along `dir=`) |

`stretch` on `align=` is the only value that changes a child's cross-axis *size* rather than its position — used when every child in a row should share the row's full height (a toolbar's buttons, for instance).

## 2. Sizing keywords, applied

Every `stack`/`grid` child resolves to `hug`, `fill`, or `fixed(space-tier)` along the container's axis (`02-style/07-layout-and-spacing.md` §5). A row with one `fill` label and one `hug` action button is the standard shape of almost every list row and form field in this system — the label consumes whatever space the action doesn't need, rather than either side hardcoding a width.

## 3. Per-component adaptation across size classes

| Component | `compact` | `standard` | `expansive` |
|---|---|---|---|
| `NavList` | Collapses to icon-only (`label=` still required and still resolves for assistive technology — visual collapse never removes the accessible label) | Icon + label | Icon + label, MAY gain a persistent secondary detail line |
| `AppBar` | Overflow actions collapse into a single `Menu` behind an overflow `IconBtn` | Actions render inline up to a fixed count before overflowing | All actions render inline |
| `TabBar` / `SegmentedControl` | Scrolls horizontally if it would otherwise overflow, rather than shrinking labels below `body-small` | Fits without scrolling in the common case | Fits without scrolling |
| A form's `FieldGroup` sequence | Single column | Single or two-column, content-dependent | Two-column where fields group logically (e.g. city/region/postal code) |
| `grid` | 4 columns (`02-style/07-layout-and-spacing.md` §2) | 8 columns | 12 columns |
| `Sheet` | Occupies the full surface (behaves like a modal `dialog` regardless of its declared modality) | Anchored panel, partial width | Anchored panel, fixed width, content behind it remains fully visible and interactive if non-modal |

A component that does not appear in this table and has no adaptation of its own (most `button`, `field`, `toggle` instances) is expected to size via §2's keywords and needs no special-cased breakpoint behavior — the fact that most primitives need nothing here is the intended outcome of Principle 1 and 8, not a gap in this table.

## 4. Nesting discipline

- A `stack` wrapping a single child with no `Surface` mixin, no distinct `gap`/`pad`, and no alignment different from what the parent already provides is not adding structure — it's adding a layer a reviewer has to look through to find the real structure. Remove it.
- Prefer flattening: three peer `stack`s inside one parent `stack` read more clearly than one `stack` containing a `stack` containing a `stack`, when the visual result is identical either way.
- A new named component (`03-widgets-and-types/02-component-library.md`) is the right answer once a specific nesting pattern (a label+field+help arrangement, an icon+text+chevron row) repeats three or more times across a codebase's screens — at that point the repetition itself is the signal, not a stylistic judgment call. This is Principle 8 applied concretely: composition first, and a small, deliberate vocabulary expansion once a pattern actually repeats.

## 5. The empty-container check

Every `stack.Surface` (or any container-carrying node) SHOULD be able to answer Principle 1's question — "what would be ambiguous without this container" — and this document adds the practical version of that check: if removing the container and promoting its children up one level changes nothing about how the screen reads, remove it. A container earns its place by doing one of: establishing a boundary between unrelated content, carrying an elevation/vibrancy signal that matters, or being the thing a `Hoverable`/`Pressable`/`Selectable` state is actually attached to.

---

*Cross-references: `01-hig/01-design-principles.md` (Principles 1, 8), `02-style/07-layout-and-spacing.md` (size classes, grid, sizing keywords this file applies), `03-widgets-and-types/01-primitive-types.md` §1–§2 (`stack`/`grid` props), `03-widgets-and-types/02-component-library.md` (when repetition should become a named component), `01-hig/04-inclusive-and-adaptive-design.md` §3 (size-class rationale).*
