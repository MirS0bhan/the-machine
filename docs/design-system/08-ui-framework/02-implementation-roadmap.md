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

| Item | Module / API |
|---|---|
| ASL subset | `ui-runtime/src/asl.rs` — tokens/scales/styles used by boot + Surface/Focusable/Pressable |
| Scroll + clip | `compositor/src/clip.rs`, `ui-runtime/src/scroll.rs`, `list` primitive paint |
| Widgets | `toggle`, `slider` (incl. progress), `icon` bitmaps, basic `list` rows |
| Dialog | `dialog` primitive + scrim; reuse confirmation exclusivity pattern (not `e4`) |
| Clipboard | `clipboard.get`/`set` via system-daemon; Ctrl-C/V on field |
| Damage | Dirty-rect present |

## P2 — Parity areas

Motion runtime, canvas/chart/media, AT-SPI a11y tree, DnD, i18n/RTL/IME, xdg-shell app hosting (G17), full component registry port from `ui-engine/components.py`.

## Explicit non-goals (near term)

Matching Qt/GTK widget count, CSS engines, GPU vibrancy shaders, XWayland, or Compose lazy-list virtualization.
