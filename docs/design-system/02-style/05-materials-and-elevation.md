# Materials and Elevation

Elevation is a hierarchy signal (Principle 2, `01-hig/01-design-principles.md`), not a decoration budget. This document gives the `elev` scale (`02-style/01-design-tokens.md` §3) its concrete shadow, z-order, and vibrancy behavior, and defines the required high-contrast fallback `01-hig/02-accessibility.md` §4 requires.

## 1. Elevation tiers

Each tier fixes a shadow recipe (light theme; dark theme uses the lightness-step rule from `02-style/02-color-and-surfaces.md` §1 instead of a visible shadow, per row) and a default z-order band that maps directly onto the compositor's own `z_order` field (`docs/components/compositor.md`):

| Tier | Shadow (light theme) | Dark theme signal | z-order band | Who may use it |
|---|---|---|---|---|
| `e0` | none | none | base (surface's own stacking position) | Default for everything (Principle 1) |
| `e1` | y=1px, blur=3px, 12% opacity | `surface.card` (N800), one step above canvas | 1–99 | `Card`, `ListRow`, default `Surface`-mixin containers |
| `e2` | y=4px, blur=12px, 16% opacity | `surface.raised`/`surface.floating`, two steps above canvas | 100–999 | Menus, popovers, tooltips, `SuggestionChip` flyouts |
| `e3` | y=12px, blur=32px, 24% opacity, paired with `opacity.overlay-scrim` backdrop | `surface.overlay`, three steps above canvas | 1000–9999 | `dialog` and its family (`03-widgets-and-types/02-component-library.md`) |
| `e4` | y=24px, blur=48px, 32% opacity, mandatory backdrop | Uses its own fixed palette, independent of theme (§5) | 10000 (fixed) | The Confirmation Surface only — matches the real `compositor.confirmation.set_active` z-order of 10000 exactly. No agent-composed node may request this tier; the UI Runtime MUST reject a patch that sets `elev=e4` on anything other than the reserved confirmation surface. |

Two things that are elevated to the same tier MUST look identically elevated — a `Card` at `e1` with a slightly heavier shadow than its sibling `Card` is a bug, not a stylistic accent, per Principle 2.

## 2. Vibrancy (backdrop blur)

Vibrancy is a *material*, layered on top of a surface token via the `+ vibrancy(level)` derivation (`02-style/01-design-tokens.md` §4), implemented by the compositor's blur-region mechanism (`docs/components/compositor.md` § Vibrancy / Backdrop-Blur). Three levels:

| Level | Blur radius | Backing tint | Use |
|---|---|---|---|
| `thin` | 12px | Surface token at 85% opacity over the blur | Subtle separation — a persistent status strip over varied content beneath it |
| `regular` | 24px | Surface token at 72% opacity over the blur | The default vibrant treatment — floating panels, `SuggestionChip` trays |
| `thick` | 40px | Surface token at 60% opacity over the blur | Maximum separation — a surface that must read as clearly "in front of everything," short of the reserved `e4` tier |

A vibrant surface's *opaque fallback* — the flat color it renders as when vibrancy is unavailable or disabled — is always its base surface token at full opacity, which is why `+ vibrancy()` is defined as a derivation from a base token rather than an independently authored color: the fallback is never a separate value someone has to remember to define.

## 3. Elevation and vibrancy are independent axes

A surface can be elevated without being vibrant (a plain opaque `Card` at `e1`) or vibrant without being especially elevated (a `thin`-vibrancy status strip sitting flush at `e0`). Don't reach for vibrancy as a substitute for getting the elevation tier right, and don't reach for elevation as a substitute for vibrancy when the actual goal is "let context show through," per Principle 2 — they answer different questions ("what's above what" vs. "how much of what's behind should still be visible").

## 4. High-contrast fallback (required by `01-hig/02-accessibility.md` §4)

When the high-contrast system preference is active, the UI Runtime applies this substitution automatically, with no per-component branching required:

1. Every vibrant material (§2) collapses to its opaque fallback — the base surface token at full opacity, blur removed entirely.
2. Shadow-based elevation cues (`e1`–`e3`) are supplemented with a `border.strong` (`02-style/02-color-and-surfaces.md` §5) 1px rule around the elevated surface — high-contrast mode does not rely on shadow softness alone to communicate a boundary, since shadow contrast is exactly the kind of subtle cue high-contrast users have already indicated they need reinforced.
3. `e4` (the Confirmation Surface) keeps its mandatory backdrop but renders it fully opaque rather than blurred, matching rule 1.

## 5. Why the Fallback Shell is the one exception to this whole file

The Fallback Shell's frozen-view and recovery-console UI (`docs/components/fallback-shell.md`) uses its own small, fixed palette (`#1a1a1a` background, `#ffffff` foreground, a fixed accent, a red/white "Agent Unavailable" banner) and does **not** participate in the `elev`/vibrancy/token system this file defines. This is intentional, not an oversight: the Fallback Shell exists specifically for the case where the State Store (which holds `prefs.theme.*` and everything else the token system resolves through) might itself be the thing that's unavailable or unreliable. A UI whose correctness depends on the same subsystem it exists to provide a fallback for isn't a fallback. Nothing in this document set applies to the Fallback Shell's own rendering, and nothing in the Fallback Shell's fixed palette should be read as an example of this file's token values.

---

*Cross-references: `01-hig/01-design-principles.md` (Principle 2), `01-hig/02-accessibility.md` §4–§5 (the high-contrast and reduced-transparency requirements §4 above satisfies), `02-style/01-design-tokens.md` §3–§4 (the `elev` scale and the vibrancy derivation), `02-style/02-color-and-surfaces.md` §1 (dark-theme lightness-step elevation), `docs/components/compositor.md` (the real `z_order` field and blur-region mechanism this file's numbers are chosen to match), `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §2 (the `e4` Confirmation Surface).*
