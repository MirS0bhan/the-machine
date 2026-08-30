# Design Tokens

This is the token architecture the rest of `02-style/` fills with values, and the file `01-hig/01-design-principles.md` §3 and §6 cite when it says color, spacing, radius, elevation, and motion are never raw literals in AUIL/ASL source. Three parallel systems exist, and they are not interchangeable:

| System | Declared with | Referenced with | Answers |
|---|---|---|---|
| **Token** | `token category.role = value` | `token:category.role` | "What is the *specific* value for this *specific* role?" |
| **Scale** | `scale name: tier=value tier=value ...` | `r-tier` (radius), `s-tier` (space), bare `eN` (elevation) | "Where does this value sit on a *ramp* of related values?" |
| **Motion curve** | `motion name = spring(...)` / `duration(...)` | bare `name` | "What does this transition *feel* like?" |

A token's value MAY itself be adaptive (`adaptive(light:value dark:value)`) or derived from another token plus a modifier (§4). A scale's tiers are always literal numbers — a scale is the one place a bare number is legitimate, because the scale declaration *is* the thing that turns that number into a governed value everywhere else.

## 1. Token categories

Every token name is `category.role`. This is the category registry; the full value catalog for each category lives in the file listed, except `opacity` and the fixed (non-ramp) `space.*` tokens, which are defined below because they're referenced directly by `01-hig/02-accessibility.md`.

| Category | Roles (examples) | Values defined in |
|---|---|---|
| `surface` | `canvas`, `sunken`, `card`, `raised`, `floating`, `overlay`, `inverse` | `02-color-and-surfaces.md` §1 |
| `text` | `primary`, `secondary`, `tertiary`, `disabled`, `inverse`, `on-accent`, `link`, `destructive` | `02-color-and-surfaces.md` §2 |
| `accent` | `default`, `hover`, `press`, `subtle`, `on-accent` | `02-color-and-surfaces.md` §3 |
| `status` | `positive`, `warning`, `destructive`, `info` (each with `-subtle`, `-on-subtle`, `-on-solid`) | `02-color-and-surfaces.md` §4 |
| `border` | `default`, `subtle`, `strong`, `focus` | `02-color-and-surfaces.md` §5 |
| `opacity` | `border`, `dim`, `disabled`, `overlay-scrim` | below, §2 |
| `space` | `min-target` (fixed values only — the `space` **scale**'s tiers are §3, not this category) | below, §2 |
| `icon` | `sm`, `md`, `lg`, `xl` (sizes) | `04-iconography.md` §1 |
| `type` | `family.default`, `family.numeric` | `03-typography.md` §3 |

## 2. Opacity tokens and fixed space tokens

Opacity tokens are alpha multipliers, not colors — they're applied over whatever color they're layered on (usually a neutral drawn from `text.*`), which is exactly what lets them stay correct across light, dark, and high-contrast without a separate token per mode. Per `01-hig/02-accessibility.md` §4, every one of these carries a high-contrast variant the UI Runtime substitutes automatically.

| Token | Standard | High-contrast | Used for |
|---|---|---|---|
| `opacity.border` | 12% | 100% (renders as a solid rule, not a tint) | Deriving `border.default`/`border.subtle` from a neutral (`02-color-and-surfaces.md` §5) |
| `opacity.dim` | 60% | 85% | De-emphasized decorative content that isn't disabled, just secondary |
| `opacity.disabled` | 38% | 55% | The whole content of a disabled primitive (`03-widgets-and-types/03-states-and-variants.md`) |
| `opacity.overlay-scrim` | 48% | 70% | The backdrop behind a `dialog` or overlay-tier surface |

| Token | Value | Why a token, not a scale tier |
|---|---|---|
| `space.min-target` | 44px | This is a single fixed accessibility floor (`01-hig/02-accessibility.md` §9), not a step on a ramp — it doesn't get smaller in `compact` density (`01-hig/04-inclusive-and-adaptive-design.md` §4) the way a scale tier would |

## 3. The three scales

### Scale `radius`

```
scale radius: xs=4 sm=6 md=10 lg=16 xl=24
```

| Tier | Value | Reference | Typical use |
|---|---|---|---|
| `none` | 0 | (no shorthand — set `radius=0` directly, since "no rounding" needs no ramp) | Hairline dividers, full-bleed media |
| `xs` | 4px | `r-xs` | Chips, small tags |
| `sm` | 6px | `r-sm` | Compact controls (`IconBtn`, inline chips) |
| `md` | 10px | `r-md` | Default control radius — buttons, fields |
| `lg` | 16px | `r-lg` | `Surface`-mixin containers (cards, panels) — the mixin's default |
| `xl` | 24px | `r-xl` | Large surfaces — dialogs, sheets |
| `full` | 9999px | `r-full` | Pills, avatars, fully-round icon buttons |

### Scale `space`

```
scale space: xxs=2 xs=4 sm=8 md=12 lg=16 xl=24 xxl=32 xxxl=48 huge=64
```

| Tier | Value | Reference | Typical use |
|---|---|---|---|
| `xxs` | 2px | `s-xxs` | Hairline gaps (icon-to-badge) |
| `xs` | 4px | `s-xs` | Tightest real gap — inline icon-to-label |
| `sm` | 8px | `s-sm` | Compact-density default gap |
| `md` | 12px | `s-md` | Comfortable-density default gap; default `Field` internal padding |
| `lg` | 16px | `s-lg` | Default `stack` gap; card internal padding |
| `xl` | 24px | `s-xl` | Section spacing within a surface |
| `xxl` | 32px | `s-xxl` | Spacing between unrelated regions |
| `xxxl` | 48px | `s-xxxl` | Page/surface-level top-level margins at `standard`/`expansive` size class |
| `huge` | 64px | `s-huge` | Hero/empty-state vertical rhythm |

### Scale `elev`

```
scale elev: e0=0 e1=1 e2=2 e3=3 e4=4
```

Elevation tiers are ordinal, not pixel values — the number is a stacking/shadow *tier*, and the actual shadow and z-order behavior for each tier is defined once in `02-style/05-materials-and-elevation.md` rather than here, because elevation is inseparable from the compositor's real `z_order` field (`docs/components/compositor.md`) and from vibrancy, and splitting that across two files would make the two easy to get out of sync.

| Tier | Meaning |
|---|---|
| `e0` | Flat — Principle 1's default |
| `e1` | Raised — a card or panel sitting slightly above canvas |
| `e2` | Floating — a menu, tooltip, or popover |
| `e3` | Overlay — a modal `dialog`, sitting above a scrim |
| `e4` | System — reserved for the Confirmation Surface only (`05-ui-ux-patterns/04-agent-presence-and-conversation.md` §2); no agent-composed node may request `elev=e4` |

## 4. Adaptive values and token derivation

A token's value can be a literal, an `adaptive(light:value dark:value)` pair, or a derivation from another token plus a modifier. This resolves `ui-engine/README.md` open item 3 ("token inheritance"):

```
token surface.card    = adaptive(light:#FFFFFF dark:#1C1E2B)
token surface.frosted = surface.card + vibrancy(regular)
token border.default  = text.primary + alpha(opacity.border)
```

- `+ vibrancy(level)` layers a materials treatment (`02-style/05-materials-and-elevation.md` §2) on top of a base token's color — the frosted variant is not a separately-authored color, it's the base color plus a blur/tint recipe, so it can never drift from its base as the base is retuned.
- `+ alpha(opacity-token)` derives a color at a given opacity token's alpha over the current surface, which is how `border.*` tokens stay correct across light, dark, and high-contrast without their own three-way value table (`02-color-and-surfaces.md` §5).
- A derived token MUST name its base token and modifier explicitly (as above) — a derived token defined as an independent literal that happens to match its base today is a token that will silently drift the next time the base changes.

## 5. Adding a token

A new token or scale tier is a change to the shared vocabulary every screen draws from — it goes through `07-governance/01-contribution-and-review.md`, not an ad hoc addition inside a single component's definition.

---

*Cross-references: `01-hig/01-design-principles.md` (Principles 2, 3, 6 — why tokens exist at all), `01-hig/02-accessibility.md` (§3–§4, §9 — the tokens this file is contractually required to define), `02-style/02-color-and-surfaces.md` (the `surface`/`text`/`accent`/`status`/`border` value catalog), `02-style/05-materials-and-elevation.md` (`elev` tier shadow/z-order specifics, vibrancy levels), `02-style/06-motion.md` (motion curve catalog), `07-governance/01-contribution-and-review.md` (how to add to this file).*
