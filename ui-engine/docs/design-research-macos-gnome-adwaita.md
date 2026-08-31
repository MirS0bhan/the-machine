# Design Research: macOS HIG & GNOME/libadwaita (GTK4)

**Purpose:** Extract concrete, sourced design values (color, radius, spacing,
elevation, vibrancy/materials, motion) from macOS Human Interface Guidelines
and GNOME's libadwaita/GTK4 (Adwaita) style system, and translate them into
this project's ASL (Agent Style Language) token/scale/motion/mixin model
(`ui-engine/models.py`, `ui-engine/asl_parser.py`).

The implementation of this research lives in
[`ui-engine/themes/adwaita_macos.asl`](../themes/adwaita_macos.asl), which is
loaded as the engine's default theme by `UIRuntime.load_default_theme()`
(see `ui-engine/runtime.py`).

---

## 1. Sources

- Apple Human Interface Guidelines — layout, spacing, materials/vibrancy
  (`NSVisualEffectView`), corner-radius/concentricity guidance (macOS
  Ventura → Sequoia, with Tahoe/"Liquid Glass" concentricity notes).
- GNOME `libadwaita` CSS Variables reference
  (`doc/css-variables.md`, `named-colors.html`) — UI colors, accent colors,
  window radius, palette colors.
- GNOME HIG color palette (`developer.gnome.org/hig/reference/palette.html`).
- GTK4/libadwaita animation defaults (`Adw.Animation`, `Adw.SpringAnimation`).

## 2. Color

### 2.1 GNOME / libadwaita named colors (light / dark)

| Token | Light | Dark | Notes |
|---|---|---|---|
| `window_bg_color` | `#fafafa` | `#242424` | App window background |
| `window_fg_color` | `#000000` | `#ffffff` | Primary text on window bg |
| `view_bg_color` | `#ffffff` | `#1e1e1e` | Content/list/text-view background |
| `view_fg_color` | `#000000` | `#ffffff` | Text on view bg |
| `card_bg_color` | `#ffffff` (raised) | `#303030` | `Adw.Bin.card` |
| `popover_bg_color` | `#ffffff` | `#383838` | Menus/popovers |
| `sidebar_bg_color` | `#ebebeb` | `#2a2a2a` | Adwaita sidebar/pane |
| `accent_bg_color` | `#3584e4` | `#3584e4` | Default accent ("blue") |
| `accent_fg_color` | `#ffffff` | `#ffffff` | Text/icon on accent |
| `destructive_bg_color` | `#e01b24` | `#c01c28` | Destructive actions |
| `success_bg_color` | `#2ec27e` | `#26a269` | Success state |
| `warning_bg_color` | `#e5a50a` | `#cd9309` | Warning state |
| `borders`/`--border-color` | `color-mix(currentColor 15%, transparent)` | same | Hairline borders |

GNOME palette greys used for secondary text/disabled states:
`light-1 #ffffff` … `light-5 #9a9996`, `dark-1 #77767b` … `dark-5 #000000`.

### 2.2 macOS system colors (semantic equivalents)

| macOS semantic color | Approx. sRGB (light) | Approx. sRGB (dark) |
|---|---|---|
| `windowBackgroundColor` | `#ececec` | `#292929` |
| `textBackgroundColor` (content) | `#ffffff` | `#1e1e1e` |
| `controlAccentColor` (system blue) | `#0a84ff` | `#0a84ff` |
| `labelColor` | `#000000` (100%) | `#ffffff` |
| `secondaryLabelColor` | `#000000` @ 55% | `#ffffff` @ 55% |
| `separatorColor` | `#000000` @ 10% | `#ffffff` @ 10% |

**Reconciliation:** macOS and Adwaita accent blues are close (`#0a84ff` vs
`#3584e4`); we standardize on the Adwaita accent (`#3584e4`) as the neutral
default since it reads correctly on both light/dark without per-platform
branching, and expose `accent.standalone` for a darker "pressed" tone,
matching libadwaita's `--accent-color` (a darker, `oklab`-derived standalone
variant of `--accent-bg-color`, used for text/icons instead of accent-filled
backgrounds).

## 3. Corner radius

| Context | GNOME/Adwaita | macOS HIG | Token used |
|---|---|---|---|
| Small control (checkbox, tag) | ~6px | ~6pt | `radius.sm` = 6 |
| Standard button / field | ~9–10px | ~8pt | `radius.md` = 10 |
| Card / grouped list | ~12px | ~10pt | `radius.lg` = 16* |
| Popover / menu | ~12–18px (`--window-radius` inherited) | ~12–16pt | `radius.xl` = 20 |
| Window | `--window-radius: 15px` (floating), `0` when tiled/maximized | ~10–12pt (titlebar), larger for toolbar windows (Tahoe) | `radius.window` = 15 |
| Pill / fully-rounded (switches, avatar) | `9999px` | capsule (height/2) | `radius.pill` = 999 |

*`radius.lg = 16` intentionally matches this project's original ASL example
in `docs/spec.md` (`scale radius: sm=6 md=10 lg=16`); we keep it as the
canonical "card" radius and add `xs`/`xl`/`window`/`pill` as extensions
rather than replacing the documented baseline.

**Concentricity (macOS 26 "Tahoe" / Liquid Glass):** `outer_radius =
inner_radius + padding`. Nested surfaces (e.g. a button inside a card) should
choose padding so the two corner radii stay concentric. We do not hard-code
this as a token (it's a *relationship*, not a value), but flag it as a
constraint for anyone authoring nested `Surface` mixins: pick `space.*` such
that `radius.parent - radius.child == space.used`.

## 4. Spacing

Both platforms converge on a small base unit (macOS: 4pt sub-grid /
8pt primary grid; GNOME: also effectively a 4px/8px system in
`Adw.Clamp`/`Adw.Bin` margins and `libadwaita` list-row padding).

| Token | Value | Typical use |
|---|---|---|
| `space.xxs` | 2 | Hairline gaps, separator insets |
| `space.xs` | 4 | Icon-to-label gap, tight control padding |
| `space.sm` | 8 | Control internal padding, toolbar icon spacing |
| `space.md` | 12 | Row padding, form field padding |
| `space.lg` | 16 | Standard content margin (both HIG and Adwaita default view margin) |
| `space.xl` | 20 | Section spacing |
| `space.xxl` | 24 | Window/view side margins, large section gaps |
| `space.xxxl` | 32 | Page-level top/bottom margins |

## 5. Elevation / depth

Both design systems are largely **flat**: elevation is communicated with a
thin border + very soft shadow rather than strong drop shadows (contrast
with Material Design). We model elevation as a small integer scale that
mixins bump on hover/press, letting the renderer decide the actual
blur/spread in device pixels:

| Token | Meaning | Approx. shadow |
|---|---|---|
| `elev.e0` | Flat / resting | none |
| `elev.e1` | Card / grouped box | `0 1px 2px rgba(0,0,0,.12)` |
| `elev.e2` | Hovered row/card | `0 2px 6px rgba(0,0,0,.16)` |
| `elev.e3` | Raised button / dropdown | `0 4px 10px rgba(0,0,0,.20)` |
| `elev.e4` | Popover / menu / sheet | `0 8px 24px rgba(0,0,0,.28)` |

## 6. Materials / vibrancy

- **macOS `NSVisualEffectView.Material`:** `sidebar`, `menu`, `popover`,
  `headerView`, `underWindowBackground` — translucent, blurred backdrops.
  Vibrancy blends foreground/background color so grayscale glyphs gain
  contrast; Apple recommends enabling vibrancy only on leaf views.
- **GNOME:** Adwaita sidebars/headerbars use a subtle flat tint rather than
  blur by default (blur is compositor-provided, e.g. via `gnome-shell`, not
  a widget-level primitive); GTK4 leaves backdrop blur to the shell/window
  manager.
- We expose a `vibrancy` property on tokens/mixins (already modeled by
  `DesignToken.vibrancy` in `models.py`) with values `"sidebar"` and
  `"popover"`, which a compositor-side renderer can map to an actual
  backdrop-blur + tint effect; on platforms without compositor blur it
  degrades gracefully to the flat `surface.tertiary`/`surface.overlay`
  color.

## 7. Motion

| Curve | Platform reference | Encoding in ASL |
|---|---|---|
| `snappy` | macOS default UI spring feel; matches this project's own `docs/spec.md` example (`spring(stiffness=300 damping=26)`) | `motion snappy = spring(stiffness=300 damping=26)` |
| `gentle` | Softer spring for larger surfaces (sheets, popovers) | `motion gentle = spring(stiffness=180 damping=20)` |
| `adwaita` | `Adw.Animation`/`Adw.TimedAnimation` default: ~200ms, ease-out-cubic | `motion adwaita = duration(200ms ease=ease-out-cubic)` |
| `press` | Fast down-state feedback used by both platforms for press/release | `motion press = duration(100ms ease=ease-out)` |

## 8. Widgets → mixins

| Widget concept | macOS | GNOME/Adwaita | ASL mixin(s) implemented |
|---|---|---|---|
| Flat container | any `NSView` w/ background | `Adw.Bin` | `Surface` |
| Hover feedback | subtle highlight on rows/buttons | `:hover` state, `row.activatable` | `Hoverable` |
| Press feedback | control "pushed" state | `:active` state | `Pressable` |
| Grouped card | boxed content, subtle shadow | `.card` style class | `Card` |
| Filled/accent button | default button (`NSButton` `.bezelStyle = .rounded`, key/accent) | `.suggested-action` | `PrimaryButton` |
| Round icon-only button | toolbar icon button | `.flat.circular` button | `IconBtn` |
| Text input | `NSTextField` w/ bezel | `Adw.EntryRow` / `GtkEntry` | `Field` |
| List/table row | `NSTableView` row | `Adw.ActionRow` / `GtkListBox` row | `ListRow` |
| Sidebar pane | source list w/ sidebar material | `Adw.NavigationSplitView` sidebar | `Sidebar` |
| Popover/menu | `NSPopover` | `Gtk.Popover` | `Popover` |
| Switch | `NSSwitch` | `Gtk.Switch` | `Toggle` |
| Slider | `NSSlider` | `Gtk.Scale` | `Slider` |

Each mixin above is implemented as a `style` block in
`ui-engine/themes/adwaita_macos.asl`, composed from the tokens/scales/motions
defined in sections 2–7. See that file for the literal ASL source, and
`ui-engine/runtime.py::UIRuntime.load_default_theme` for how it's wired into
the running UI Engine.
