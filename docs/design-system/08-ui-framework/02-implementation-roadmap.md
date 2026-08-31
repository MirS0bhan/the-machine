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
| Widgets | toggle/slider/list/dialog; geometric `icon`; media/chart deferred | landed (icons geometric only) |
| Dialog | scrim + soft exclusivity; Escape dismiss | landed |
| Clipboard | `clipboard.get`/`set`; Ctrl-C/V/X | landed (in-memory) |
| Damage | Dirty-rect present | landed |
| Hover | pointer `move` → `hovered` prop + press chrome | landed (follow-on) |
| Caret paint | focused field caret glyph | landed (follow-on) |
| Slider input | click x → `value` | landed (follow-on) |
| MCP registry | `ui.auil.*` + focus/theme on mcp-bus | landed (follow-on) |

## P2 — Parity areas

Motion runtime, canvas/chart/media bitmaps, real grid algorithm, AT-SPI a11y tree, DnD, i18n/RTL/IME, OS clipboard, xdg-shell app hosting (G17), full component registry port from `ui-engine/components.py`, focus trap for dialogs.

## Explicit non-goals (near term)

Matching Qt/GTK widget count, CSS engines, GPU vibrancy shaders, XWayland, or Compose lazy-list virtualization.
