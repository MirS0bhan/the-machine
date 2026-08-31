# Implementation Roadmap

Concrete modules for closing toolkit gaps on the AUIL → MCP → compositor spine.

## P0 — Interactive shell (must land for real apps)

| Item | Module / API | Done when |
|---|---|---|
| HarfRust fonts | `compositor/src/text.rs`, `assets/fonts/` | `compositor.status.text == "harfrust+freetype"`; Inter title/body/label |
| Keyboard forward | `system-daemon/src/input.rs` | `EV_KEY` → `compositor.input` `{event:key}` |
| Focus model | `ui-runtime/src/focus.rs`, `ui.focus.*` | Tab order among interactive nodes; click focuses; Enter activates button |
| Text editing | `ui-runtime/src/input_edit.rs` | Focused `field` accepts insert/backspace; caret in props; `on:change` |
| Surface update | `compositor.surface` `action=update` | Patches don't only create; orphans destroyed |
| Local press feedback | layout/style hover/press | Border/bg swap without waiting on agent |

## P1 — Framework breadth

| Item | Module / API | Status |
|---|---|---|
| ASL subset | `ui-runtime/src/asl.rs` — tokens/scales used by boot | landed |
| Scroll + clip | `clip.rs`, `scroll.rs`, list paint | landed |
| Widgets | toggle/slider/list/dialog; geometric `icon` | landed |
| Dialog | scrim + soft exclusivity; Escape dismiss | landed |
| Clipboard | `clipboard.get`/`set`; Ctrl-C/V/X | landed |
| Damage | Dirty-rect present | landed |
| Hover / caret / slider click / `ui.auil.*` | follow-on honesty pass | landed |

## P2 — Parity areas

| Item | Module / API | Status |
|---|---|---|
| Real grid | `ui-runtime/src/grid.rs` + layout `cols`/`col_span`/`rtl` | landed |
| Media / chart paint | compositor `paint_media` / `paint_chart` | landed (procedural; no video decode) |
| Motion runtime | opacity tween in present-loop; snappy/gentle/reduced | landed (opacity only) |
| A11y tree | `ui.a11y.tree` AT-SPI-shaped MCP export | landed (no AT-SPI D-Bus yet) |
| DnD | `draggable` + drag/drop → `change` | landed |
| RTL | `dir=rtl` / `rtl=true` on stack & grid | landed (layout mirror) |
| OS clipboard | wl-copy/xclip/xsel best-effort + memory | landed |
| Dialog focus trap | Tab scoped to dialog subtree | landed |
| Component registry | `ui.components.list` subset | landed (names/status; not full recipe expand) |
| xdg-shell (G17) | third-party Wayland clients | **absent** — `compositor.status.xdg_shell="absent"` |
| IME / full i18n catalogs | locale + input method | deferred |
| AT-SPI D-Bus bridge | system a11y bus | deferred (MCP tree is the interim) |

## Explicit non-goals (near term)

Matching Qt/GTK widget count, CSS engines, GPU vibrancy shaders, XWayland, or Compose lazy-list virtualization.
