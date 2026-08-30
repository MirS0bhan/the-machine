# Iconography

Icons are rendered by the `icon` primitive (`03-widgets-and-types/01-primitive-types.md` §9). This document defines the grid every icon asset is drawn against and the rules governing how icons are sized, colored, and combined with text.

## 1. Grid and sizes

Every icon is drawn on a 24×24 canvas with an effective live area of 20×20 (a 2px margin on all sides) at a stroke weight of 1.75px, so that icons of different visual complexity still read as the same "weight" of mark sitting next to each other. Corner joins round to `r-xs` (`02-style/01-design-tokens.md` §3) — sharp enough to stay crisp at small sizes, rounded enough to match the system's general shape language.

| Token | Rendered size | Typical use |
|---|---|---|
| `icon.sm` | 16px | Dense contexts — inline in `caption`-scale text, compact `list` rows |
| `icon.md` | 20px | Default — inline with `body`/`label`-scale text, most `button`/`IconBtn` content |
| `icon.lg` | 24px | Standalone controls, `AppBar` actions, nav items |
| `icon.xl` | 32px | Empty-state and feature-level marks (paired with `heading`/`title-3` text, never smaller) |

An icon rendered at any size other than these four tokens is a scale violation, not a design choice — icons scaled arbitrarily lose the stroke-weight consistency the whole set depends on.

## 2. Style: outline by default, filled for active state

Icons render **outline** style by default and switch to a **filled** variant only to represent an active/selected state (`state:selected → variant=filled`, `03-widgets-and-types/03-states-and-variants.md`) — a filled navigation icon means "you are here," never decoration. An icon MUST NOT be filled and outline at the same time to indicate emphasis; emphasis is a size or color change, not a style-family change.

## 3. Color

Icons never carry a hardcoded fill. An icon's color is always one of:

- Inherited from the surrounding `text.*` token (the common case — an icon sitting next to a label matches that label's color exactly, including its `hover`/`disabled` state transitions, because it's the same token reference, not a separately-tracked color).
- An explicit `status.*` token, when the icon *is* the status indicator (a destructive icon next to a destructive action).
- `accent.default`, when the icon represents the interactive/selected state of a control.

## 4. Directionality

Per `01-hig/04-inclusive-and-adaptive-design.md` §2, an icon that encodes a direction (a "next" chevron, a "back" arrow, a progress arrow) MUST mirror under right-to-left layout; a symbolic icon (settings gear, a status icon, a trash icon) MUST NOT mirror — mirroring a symbol that carries no directional meaning just makes it look subtly wrong to anyone used to seeing it the other way.

## 5. Icons are never the sole carrier of meaning

Per Principle 7 (`01-hig/01-design-principles.md`) and `01-hig/02-accessibility.md` §1 (icon-only controls require an explicit `label=`), an icon MUST always be backed by one of: a visible text label alongside it, an accessible `label=`, or (for status specifically) a color+text pairing per `02-style/02-color-and-surfaces.md` §4. An icon that is the *only* signal for something — no label, no paired text, no tooltip — is a defect regardless of how "obviously recognizable" the icon seems to its author.

## 6. Badges and status dots

A small status dot or count badge MAY overlay the bottom-trailing or top-trailing corner of an `icon.lg`/`icon.xl` icon (never `icon.sm`/`icon.md` — there isn't room to do it without the badge overwhelming the icon). Badge colors are always `status.*` tokens; a count badge's number uses `type.family.numeric` (`02-style/03-typography.md` §4) once it exceeds a single digit.

## 7. Motion

Icon state changes (outline↔filled on selection, a status icon updating) cross-fade using `motion.snappy` (`02-style/06-motion.md`) rather than a literal shape morph — morphing between two genuinely different icon shapes reads as glitchy, not smooth, unless the icon was purpose-built as an animated pair (e.g. a play/pause toggle, which is the one sanctioned case for a designed morph rather than a cross-fade).

---

*Cross-references: `03-widgets-and-types/01-primitive-types.md` §9 (the `icon` primitive), `02-style/01-design-tokens.md` (`icon.*` size tokens), `02-style/02-color-and-surfaces.md` (`status.*`/`text.*` tokens icons inherit), `01-hig/04-inclusive-and-adaptive-design.md` §2 (mirroring rules), `01-hig/02-accessibility.md` §1 (icon-only labeling).*
