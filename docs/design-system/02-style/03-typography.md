# Typography

The `text` primitive (`03-widgets-and-types/01-primitive-types.md` §3) takes exactly two style-relevant props: `role=` (fixed, four values, accessibility-load-bearing) and `size=` (the visual scale step). This document defines both axes and why they're separate.

## 1. Why role and size are two different props

`text`'s `role=` prop is a **closed, four-value enum** — `title`, `body`, `caption`, `label` — because it is read by assistive technology to determine document structure (a heading vs. a paragraph vs. a caption vs. a form label), and a four-value enum is exactly as much semantic structure as this system needs to expose. `size=` is a **ten-step visual scale** — it controls how big the text renders, independent of what role it plays in the document's structure. A section heading and a hero headline are both `role=title`; they differ only in `size`.

```
text(role=title size=display) "Welcome back"
text(role=title size=title-3) "Recent activity"
text(role=body size=body) "Your download finished a few minutes ago."
text(role=caption size=caption) "Last updated 2 minutes ago"
```

This is a deliberate reconciliation: earlier drafts of this system's accessibility rules referred to scale steps like `title-1` as if they were roles in their own right. They are `size=` values scoped to `role=title`; `role` itself never grows past its four accessibility-relevant values.

## 2. The scale

| `size=` | Role it's typically used with | Pixels | Line height | Weight |
|---|---|---|---|---|
| `display` | `title` | 32px | 40px | `bold` (700) |
| `title-1` | `title` | 26px | 32px | `bold` (700) |
| `title-2` | `title` | 20px | 26px | `bold` (700) |
| `title-3` | `title` | 17px | 22px | `bold` (700) |
| `heading` | `title` (a smaller in-content heading, not a surface title) | 15px | 20px | `medium` (500) |
| `body-large` | `body` | 16px | 24px | `regular` (400) |
| `body` | `body` | 14px | 20px | `regular` (400) |
| `body-small` | `body` | 13px | 18px | `regular` (400) |
| `caption` | `caption` | 12px | 16px | `regular` (400) |
| `label` | `label` | 13px | 16px | `medium` (500) |

`title-2` (20px/`bold`) and `body` (14px/`regular`) are the two values already fixed by the reference theme example in `docs/components/ui-runtime.md`; every other step is built around those two anchors.

`title-1` and above are the "large-scale text" threshold `01-hig/02-accessibility.md` §3 grants the relaxed 3:1 contrast ratio; `title-2` and `title-3` also qualify because they are still bold and at least 17px. `heading` does **not** qualify for the relaxed ratio — it's a `title`-role text node but sized and weighted closer to body text, so it holds to the 4.5:1 body threshold.

## 3. Weight scale

Three named weights, no others:

| Name | Numeric | Use |
|---|---|---|
| `regular` | 400 | Default for `body`/`caption` |
| `medium` | 500 | `label`, `heading`, and any control text that needs to stand slightly apart from surrounding body text without reading as a heading |
| `bold` | 700 | All `title`-role sizes |

A `text` node MUST NOT set an arbitrary numeric weight outside this set — matching Principle 3's ban on raw literals, applied to weight specifically.

## 4. Type families

| Token | Reference typeface | Use |
|---|---|---|
| `type.family.default` | Inter | Every `text` node by default |
| `type.family.numeric` | A metrically-compatible monospace companion face, tabular-figure variant of the default family where available | Timers, counters, tabular data, file sizes — anywhere digits must align column-to-column across rows |

`type.family.numeric` is a **feature**, not a separate font choice, wherever the default face supports tabular figures — the point is fixed-width digits, not a visually distinct typeface. Use it any time digits appear in a `list` column, a countdown, or anywhere per `01-hig/03-content-and-voice.md` §5.

## 5. Measure and wrapping

- Body text SHOULD wrap at a measure (line length) of roughly 50–75 characters at `standard` size class; a `text` node that's allowed to stretch to the full width of an `expansive` surface without a max-width constraint produces lines that are tiring to read and is a layout bug, not a content bug.
- `title`-role text MAY exceed that measure — headlines read fine wider than body copy does.
- Per `01-hig/04-inclusive-and-adaptive-design.md` §1, no container sized to a specific *character count* of the source-language string is acceptable; size to content/available space, not to a string-length assumption.

## 6. Numeric and tabular figures

Per `01-hig/03-content-and-voice.md` §5, any place several numbers stack vertically (a `list` of file sizes, a countdown, a settings value column) MUST use `type.family.numeric` so the digits align. A single inline number in a sentence does not need it.

---

*Cross-references: `01-hig/02-accessibility.md` §3 (the `title-1`-and-above large-text threshold this file defines), `01-hig/03-content-and-voice.md` §2, §5 (mechanics and numerals that typography renders), `03-widgets-and-types/01-primitive-types.md` §3 (the `text` primitive's `role=`/`size=` props), `02-style/01-design-tokens.md` (`type.family.*` token category).*
