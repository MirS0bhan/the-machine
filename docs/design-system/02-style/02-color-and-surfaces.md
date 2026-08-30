# Color and Surfaces

Every color in The Machine's UI resolves through a token (`01-design-tokens.md`); nothing here is a suggestion an agent could reasonably substitute a "close enough" literal for. This document defines the reference neutral ramp, and the `surface`, `text`, `accent`, `status`, and `border` token catalogs built from it.

## 0. The neutral ramp

All neutral tokens derive from one hue so that light and dark surfaces feel like the same system rather than two unrelated palettes. The reference ramp uses a cool, low-saturation neutral (hue 234°, ~14% saturation) rather than a pure gray, which is what keeps white-feeling surfaces from reading as sterile and dark surfaces from reading as flat black.

| Step | Approx. HSL | Hex |
|---|---|---|
| N0 | 234° 20% 100% | `#FFFFFF` |
| N10 | 234° 20% 98% | `#F7F8FC` |
| N25 | 234° 16% 94% | `#EDEEF5` |
| N50 | 234° 14% 88% | `#DCDEE9` |
| N100 | 234° 12% 80% | `#C4C7D6` |
| N200 | 234° 10% 68% | `#A2A6BB` |
| N300 | 234° 9% 56% | `#82869C` |
| N400 | 234° 9% 44% | `#656980` |
| N500 | 234° 10% 36% | `#4F5268` |
| N600 | 234° 12% 28% | `#3B3E52` |
| N700 | 234° 14% 20% | `#292B3C` |
| N800 | 234° 16% 14% | `#1C1E2B` |
| N900 | 234° 18% 9% | `#12131C` |
| N950 | 234° 20% 6% | `#0B0C13` |
| N1000 | 234° 22% 3% | `#05050A` |

These hex values are the canonical reference implementation; a visual QA pass MAY fine-tune individual steps, but any change is a change to this table, not a per-screen override.

## 1. Surface tokens

**Rule for elevation and color, stated once:** in light theme, elevation is communicated primarily through *shadow* (`05-materials-and-elevation.md`), because light surfaces stacked on light surfaces don't differentiate well through color alone. In dark theme, elevation is communicated primarily through a *lightness increase per tier*, because cast shadow barely reads against a dark canvas. This is why the table below holds surface fills flat across elevation in light theme but steps them up in dark theme — that asymmetry is intentional, not an inconsistency.

| Token | Light | Dark | Notes |
|---|---|---|---|
| `surface.canvas` | `#F7F8FC` (N10) | `#0B0C13` (N950) | The root background beneath everything |
| `surface.sunken` | `#EDEEF5` (N25) | `#05050A` (N1000) | Recessed wells — input backgrounds, code/log panes |
| `surface.card` | `#FFFFFF` (N0) | `#1C1E2B` (N800) | `elev=e1` default — differentiated from canvas by shadow in light, by lightness in dark |
| `surface.raised` | `#FFFFFF` (N0) | `#292B3C` (N700) | `elev=e2` |
| `surface.floating` | `#FFFFFF` (N0) | `#3B3E52` (N600) | `elev=e2`–`e3` transient surfaces (menus, tooltips, popovers) |
| `surface.overlay` | `#FFFFFF` (N0) | `#292B3C` (N700) | The `dialog` body itself, `elev=e3` |
| `surface.inverse` | `#12131C` (N900) | `#F7F8FC` (N10) | Contrast call-outs (a light-theme tooltip that intentionally reads as "flipped") |

`surface.overlay-scrim` is not a fill color — it's `surface.inverse` at `opacity.overlay-scrim` (`01-design-tokens.md` §2), which is why it darkens correctly behind a dialog in both themes without its own entry here.

## 2. Text tokens

| Token | Light | Dark | Notes |
|---|---|---|---|
| `text.primary` | `#12131C` (N900) | `#F7F8FC` (N10) | Default reading text |
| `text.secondary` | `#3B3E52` (N600) | `#A2A6BB` (N200) | Supporting text, metadata |
| `text.tertiary` | `#656980` (N400) | `#82869C` (N300) | Least-emphasized legible text (timestamps, counts) |
| `text.disabled` | `#82869C` (N300) | `#4F5268` (N500) | Paired with `opacity.disabled` on the whole control, not used standalone |
| `text.inverse` | `#F7F8FC` (N10) | `#12131C` (N900) | Text on an `inverse` surface |
| `text.on-accent` | `#FFFFFF` | `#12131C` (N900) | See §3 — **this flips between themes**, it is not always white |
| `text.link` | resolves to `accent.default` | resolves to `accent.default` | Never a separate hue from the interactive accent |
| `text.destructive` | resolves to `status.destructive` | resolves to `status.destructive` | For destructive-emphasis inline text (not buttons — those use `status.destructive` as a fill) |

## 3. Accent tokens

`accent` is the one token category a person can personalize (`01-hig/04-inclusive-and-adaptive-design.md` §7) — the *hue* is user-selectable; every derived accent token updates together because they're all defined relative to that one hue, never as independent literals.

| Token | Light (reference hue 262°) | Dark (reference hue 262°) |
|---|---|---|
| `accent.default` | `#6C3CE0` | `#9C7CF2` |
| `accent.hover` | `#5C2FC9` | `#A98EF5` |
| `accent.press` | `#4E27AD` | `#8666E8` |
| `accent.subtle` | `#EFE7FB` | `#2B1F4A` |
| `accent.on-accent` | `#FFFFFF` | `#12131C` (N900) |

**Why `accent.on-accent` is dark text in dark theme:** dark-theme accent fills are deliberately lightened (`#9C7CF2` rather than `#6C3CE0`) so they read clearly against a dark canvas — but that same lightening means a *light* foreground no longer has enough contrast against them. The correct on-accent foreground in dark theme is the dark neutral, not white. Treating "on-accent is always white" as a shortcut is the single most common token-pairing mistake this system's tokens are designed to make structurally impossible, as long as `text.on-accent`/`accent.on-accent` are resolved from the token, never hardcoded.

## 4. Status tokens

Four semantic hues, each with a default (solid-fill), subtle (tint background), and two foreground pairings. The same dark-theme "on-solid flips to a dark foreground" rule from §3 applies here identically, for the identical reason.

| Status | Hue | Light default | Dark default | Light subtle bg | Dark subtle bg |
|---|---|---|---|---|---|
| `status.positive` | 152° | `#1E9E6B` | `#4CD98A` | `#E3F5EC` | `#163826` |
| `status.warning` | 38° | `#8A5200` | `#F5A93D` | `#FBF0DC` | `#3A2C10` |
| `status.destructive` | 6° | `#C42B2B` | `#FF6B61` | `#FBE7E7` | `#3A1616` |
| `status.info` | 202° | `#0E72B5` | `#5CB8F5` | `#E4F1FA` | `#142F3E` |

Derivation rule (applies to all four, so it's stated once instead of sixteen more times):

- `status.<name>-on-subtle` = `status.<name>`'s **default** value, used as text/icon color on top of that status's own `-subtle` background. It is contrast-safe by construction because the subtle background is the same hue at extreme lightness.
- `status.<name>-on-solid` = `#FFFFFF` in light theme, `text.primary`'s dark-theme value (N900) in dark theme — following §3's flip rule, because every dark-theme status default above is deliberately light enough to read on a dark canvas, which means it is not dark enough for white text on top of it.

## 5. Border tokens

Borders are derived, not independently authored, so they can never drift out of sync with the neutral ramp they're built from:

| Token | Derivation | Resolves to (light / dark) |
|---|---|---|
| `border.default` | `text.primary` at `opacity.border` (12%) | a 12%-alpha hairline of N900 / N10 |
| `border.subtle` | `text.primary` at half of `opacity.border` (6%) | a fainter hairline, for internal separators that need to be present but not noticed |
| `border.strong` | `text.primary` at full opacity (solid) | a fully opaque rule — used sparingly, mainly in high-contrast mode where `opacity.border` itself resolves to 100% anyway |
| `border.focus` | `accent.default`, solid, no alpha | Focus rings MUST be fully opaque and MUST use the accent hue, never a neutral — a dim focus ring defeats the purpose of `01-hig/02-accessibility.md` §6 |

## 6. Contrast reference

Every pairing an agent is expected to reach for by default meets `01-hig/02-accessibility.md` §3's thresholds in both themes:

| Surface | Text token | Meets 4.5:1 (body) | Meets 3:1 (large/`title-1`+) |
|---|---|---|---|
| `surface.canvas` | `text.primary` | ✓ | ✓ |
| `surface.canvas` | `text.secondary` | ✓ | ✓ |
| `surface.card` | `text.primary` | ✓ | ✓ |
| `surface.card` | `text.secondary` | ✓ | ✓ |
| `surface.card` | `text.tertiary` | — | ✓ (large-scale use only) |
| `accent.default` (solid fill) | `accent.on-accent` | ✓ | ✓ |
| `status.*` (solid fill) | `status.*-on-solid` | ✓ | ✓ |
| `status.*-subtle` (tint fill) | `status.*-on-subtle` | ✓ | ✓ |

`text.tertiary` on any surface is validated for large-scale text only (`01-hig/02-accessibility.md` §3) — an agent placing `text.tertiary` at `body` scale or smaller is making a contrast error even though the token itself is "correct" in isolation; this is exactly the kind of token-pair-not-token error §3 of `01-hig/02-accessibility.md` calls out.

## 7. Theme mode resolution

Theme mode (`light` / `dark` / `auto`) lives at `prefs.theme.mode` in the State Store. Changing it is an ordinary `state.set` write; every `adaptive(light:.. dark:..)` token in the live UI tree re-resolves because the UI Runtime's token resolver already holds a `state.watch` subscription on the active theme path, the same reactive mechanism any other `@`-bound value uses. This resolves `ui-engine/README.md` open item 7 ("compositor-level theming — system dark/light switch broadcast"): no separate broadcast channel is needed, because theme mode is State Store data like any other, and every token consumer is already watching it.

`auto` resolves from the System Daemon's ambient light/time-of-day signal where available, and otherwise defaults to `light`; a person can always override to an explicit `light` or `dark` regardless of what `auto` would have chosen.

---

*Cross-references: `01-hig/02-accessibility.md` (§3–§4 contrast and high-contrast requirements this file satisfies), `01-hig/04-inclusive-and-adaptive-design.md` §7 (accent personalization), `02-style/01-design-tokens.md` (the token/derivation mechanism this file's values plug into), `02-style/05-materials-and-elevation.md` (how `surface.*` combines with elevation and vibrancy), `05-ui-ux-patterns/02-feedback-and-status.md` (status token usage in components).*
