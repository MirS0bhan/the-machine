# Inclusive and Adaptive Design

Accessibility (`02-accessibility.md`) is about making a given screen usable with assistive technology. This document is about making the *same* AUIL/ASL source produce a correct screen across languages, reading directions, surface sizes, and input modalities the original author may never have tested against. Both are Principle 5 in spirit — correct by construction rather than by review — applied to different axes of variation.

## 1. Internationalization readiness

1. All agent-composed and system-authored text MUST flow through locale-aware formatting for dates, times, numbers, and units (`01-hig/03-content-and-voice.md` §5) rather than hand-assembled strings — a string built by concatenating translated fragments around a number almost never reorders correctly across languages.
2. Text content MUST NOT be baked into an `icon` or `image` asset. Anything a person reads has to be a `text` node so it can be translated, resized, and read by assistive technology; an icon that *looks like* a word is not text.
3. Fixed-width containers around text are a bug, not a layout choice. Every primitive that wraps a `text` child MUST size to its content or wrap, never clip translated text that runs 30–40% longer than its source-language equivalent — a common ratio for translation from a terse source language into a more verbose target one.
4. `label=` props, `announce=` regions, and any other structural (non-visual) string MUST be part of the same localization pipeline as visible text — an accessible label that's correct in one language and forgotten in translation is a §2/§1 violation of `02-accessibility.md`, not a separate bug class.

## 2. Bidirectional layout

The layout system is written in **logical properties**, never physical ones, for exactly this reason:

| Logical term (use this) | Physical equivalent it replaces | Why |
|---|---|---|
| `start` / `end` | `left` / `right` | Flips automatically under a right-to-left writing direction; a literal `left`/`right` prop would require every layout to be authored twice |
| `inline` axis | horizontal axis | The axis text flows along — horizontal in most scripts, but not universally |
| `block` axis | vertical axis | The axis blocks stack along |

**What flips under a right-to-left writing direction:**
- `stack` alignment and `gap` distribution along the inline axis.
- Icons that encode directionality — a "next" chevron, a "back" arrow, a progress-direction indicator.
- The reading order of a `list`'s items along the inline axis (row-oriented lists only; vertically-stacked lists are unaffected).

**What does NOT flip, regardless of writing direction:**
- Numerals and numeric formatting.
- Media transport controls (play/pause/skip) — these follow a left-to-right timeline convention independent of text direction, since they represent time, not text.
- A `chart`'s data axes, unless the chart is explicitly a right-to-left-native visualization type.
- Any icon that is symbolic rather than directional (a status icon, a settings gear).

A component that hardcodes `align=left` or a `padding-left` prop instead of `align=start` / `padding-start` MUST be treated as a layout bug, not a style preference — this is checkable the same way a raw hex value is checkable (Principle 3, `01-design-principles.md`).

## 3. Adaptive layout across surface sizes

The Machine does not assume one fixed surface size. A `stack`/`grid` layout resolves against one of three size classes based on the available inline-axis space of the surface it renders into (`03-widgets-and-types/04-composition-and-responsive-layout.md` defines the exact breakpoints and per-component adaptation rules):

| Size class | Typical context |
|---|---|
| `compact` | A narrow panel, a split-view secondary pane, a small auxiliary surface |
| `standard` | A typical single-purpose surface at its default size |
| `expansive` | A maximized or large-format surface, multi-column content |

Components MUST degrade gracefully at `compact` (collapsing a `NavList` to icons-only, stacking a horizontally-laid-out form into a single column) rather than clipping or requiring horizontal scroll for primary content. Horizontal scroll is acceptable only for content that is inherently a single inline-axis sequence (a `list` of chips, a media filmstrip) — never for a form or a paragraph of text.

## 4. Density and input modality

- Two density presets exist: `comfortable` (default) and `compact` (denser spacing and smaller `space` scale tiers for information-dense contexts like a long settings list). Density changes spacing and type-scale steps only — it MUST NOT change a primitive's minimum hit target (`01-hig/02-accessibility.md` §9); `space.min-target` is invariant under density.
- The system's primary input model today is pointer, keyboard, and voice (`04-events/02-input-and-interaction.md`). Nothing in the token or component system assumes pointer-only interaction — hit targets, focus rings, and activation semantics are defined input-modality-agnostically now specifically so that adding a new input family (touch, per `ARCHITECTURE.md` §7 item 5) later is a `space.min-target` and gesture-mapping change, not a redesign.

## 5. Cognitive accessibility and plain language

- Every rule in `01-hig/03-content-and-voice.md` (concrete, front-loaded, jargon-free where a plain term exists) is also a cognitive-accessibility rule, not just a tone rule — front-loaded, concrete language reduces working-memory load for everyone, and is load-bearing for people with cognitive or attention-related disabilities.
- Progressive disclosure is the default for anything with more than one layer of complexity: show the common case, put the rest behind an explicit, clearly labeled expansion (never a hidden gesture with no visible affordance).
- Placement consistency matters more than any individual placement choice: once a pattern (where confirmation actions sit in a dialog, where an error appears relative to its field) is established in `05-ui-ux-patterns/`, every surface MUST follow it — an occasional "better" one-off placement costs more in relearning than it gains in that one instance.

## 6. Motion and vestibular sensitivity

`01-hig/02-accessibility.md` §5 defines the reduced-motion substitution as a baseline. Beyond that baseline:

- Looping or auto-playing motion (a decorative background animation, an auto-advancing carousel) is discouraged generally (Principle 1, minimal chrome) and MUST respect reduced motion by stopping outright, not just slowing down.
- Parallax-style effects (layers moving at different rates in response to scrolling or pointer position) MUST NOT be used — they are a common vestibular trigger and this system's motion vocabulary (`02-style/06-motion.md`) has no token for them, deliberately.
- Any motion triggered by something other than a direct user action (an agent-initiated patch arriving, a background task completing) SHOULD use the calmer end of the motion scale (`motion.gentle`, not `motion.snappy`) even where a user-initiated equivalent would use a snappier curve — motion the person didn't cause should never startle.

## 7. Personalization vs. system consistency

The `accent` token's hue is the one user-personalizable value in the entire token system (`02-style/02-color-and-surfaces.md` §2) — a person may change it, and every `accent.*` derived token updates consistently. Every other structural token (surfaces, elevation, spacing, radius, motion) is fixed system-wide and is **not** a personalization surface: allowing per-app or per-surface overrides of structural tokens would reintroduce exactly the "what shade of gray did we use last time" problem Principle 3 exists to eliminate, just scoped to a theme picker instead of a component author.

---

*Cross-references: `01-hig/02-accessibility.md` (the assistive-technology axis this document complements), `01-hig/03-content-and-voice.md` (localization-ready copy rules), `02-style/07-layout-and-spacing.md` (size-class breakpoints and density tokens), `03-widgets-and-types/04-composition-and-responsive-layout.md` (per-component adaptation rules), `02-style/06-motion.md` (the motion vocabulary referenced in §6).*
