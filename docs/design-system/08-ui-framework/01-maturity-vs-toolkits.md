# Maturity vs WinForms / Compose / GTK / Qt

Legend for The Machine column: **F** full · **P** partial in Rust boot path · **S** specified/docs or Python-only · **—** absent.

| Capability | WinForms | Compose | GTK | Qt | The Machine (boot) | Evidence |
|---|---|---|---|---|---|---|
| Widget catalog | F | F | F | F | **P** | Painted: `text`/`field`/`button`/`toggle`/`slider`/`list`/`dialog`/`icon` (geometric) (+ confirmation). `media`/`chart` unpainted |
| Layout | F | F | F | F | **P** | `layout.rs` stack v/h + gap + center; `grid` is a tag alias, not a grid algorithm |
| Styling / themes | F | F | F | F | **P** | Dark tokens in `tokens.rs`/`chrome.rs`; `ui.theme.*`; ASL subset (`token`/`scale` only) |
| Text / fonts | F | F | F | F | **P→F** | HarfRust + FreeType; Inter + JetBrains Mono; bitmap fallback if faces missing |
| Input events | F | F | F | F | **P** | press/click/release/move(hover)/wheel/key via system-daemon → compositor → ui.event |
| Focus | F | F | F | F | **P** | Tab order + `compositor.focus` sync; Enter activates button; Escape dismisses soft dialog |
| Accessibility | F | F | F | F | **S** | Roles/labels in HIG; no AT-SPI bridge |
| Animation | P–F | F | F | F | **S** | Motion curves in ASL docs; no present-loop tweens |
| Drawing / canvas | F | F | F | F | **P** | Rounded rects + glyphs + clip + geometric icons; no paths/images/charts |
| Data binding | F | F | P–F | F | **P** | Patch + mcp bindings; field edit; slider click→value |
| Dialogs / modals | F | F | F | F | **P** | Confirmation exclusivity (`e4`); soft `dialog` + scrim + Escape |
| Scrolling | F | F | F | F | **P** | List wheel + `scroll_y` + clip; no scrollbar chrome |
| Text editing | F | F | F | F | **P** | Caret prop + painted caret; no selection range / IME |
| Clipboard | F | F | F | F | **P** | In-memory `clipboard.get`/`set`; Ctrl-C/V/X on fields |
| DnD | F | F | F | F | **—** | Event vocab only |
| i18n / RTL / IME | F | F | F | F | **—** | HarfRust shaping only |
| Windowing | F | F | F | F | **P** | Flat z-order + damage; partial Wayland SHM; no xdg-shell (G17) |

## What "framework-complete" means here

A toolkit-level Machine UI runtime can:

1. Author every screen with the twelve AUIL primitives + ASL mixins (no ad-hoc paint).
2. Handle pointer **and** keyboard, with focus order and text editing, without an agent in the hot path for local feedback.
3. Scroll lists, show dialogs, and draw icons/charts without dropping to a foreign toolkit.
4. Expose an accessibility tree derived from primitive roles.
5. Keep confirmation chrome broker-owned and unforgeable.

Items 1–3 are P0/P1 (+ follow-on honesty pass). Items 4–5 stay P2 / policy-broker owned respectively.

## Docs vs code honesty rule

`docs/components/ui-runtime.md` and design-system specs may describe the target language. This maturity matrix plus `03-docs-code-honesty.md` are authoritative for **what boots today**. When a row moves from S/— to P/F, update both files in the same PR as the code.
