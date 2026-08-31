# Maturity vs WinForms / Compose / GTK / Qt

Legend for The Machine column: **F** full · **P** partial in Rust boot path · **S** specified/docs or Python-only · **—** absent.

| Capability | WinForms | Compose | GTK | Qt | The Machine (boot) | Evidence |
|---|---|---|---|---|---|---|
| Widget catalog | F | F | F | F | **P** | Painted: text/field/button/toggle/slider/list/dialog/icon/media/chart (+ confirmation) |
| Layout | F | F | F | F | **P** | Stack v/h + gap/center; **real grid** (`cols`/`col_span`/`rtl`) in `grid.rs` |
| Styling / themes | F | F | F | F | **P** | Dark tokens; `ui.theme.*`; ASL token/scale subset |
| Text / fonts | F | F | F | F | **P→F** | HarfRust + FreeType; Inter + JetBrains Mono |
| Input events | F | F | F | F | **P** | press/click/release/move/drag/wheel/key |
| Focus | F | F | F | F | **P** | Tab + Enter; dialog focus trap; compositor focus sync |
| Accessibility | F | F | F | F | **P** | `ui.a11y.tree` MCP export (no AT-SPI D-Bus yet) |
| Animation | P–F | F | F | F | **P** | Present-loop opacity tweens (snappy/gentle/reduced) |
| Drawing / canvas | F | F | F | F | **P** | Rounded rects + glyphs + clip + media/chart procedural paint |
| Data binding | F | F | P–F | F | **P** | Patch + mcp bindings; field edit; slider; DnD→change |
| Dialogs / modals | F | F | F | F | **P** | Confirmation e4; soft dialog + scrim + Escape + focus trap |
| Scrolling | F | F | F | F | **P** | List wheel + clip |
| Text editing | F | F | F | F | **P** | Caret paint; no IME/selection |
| Clipboard | F | F | F | F | **P** | Memory + wl-copy/xclip/xsel best-effort |
| DnD | F | F | F | F | **P** | `draggable` + drag/drop events |
| i18n / RTL / IME | F | F | F | F | **P** | RTL layout mirror; no locale catalogs / IME |
| Windowing | F | F | F | F | **P** | Flat z-order + damage; Wayland SHM; **xdg-shell absent (G17)** |

## What "framework-complete" means here

A toolkit-level Machine UI runtime can:

1. Author every screen with the twelve AUIL primitives + ASL mixins (no ad-hoc paint).
2. Handle pointer **and** keyboard, with focus order and text editing, without an agent in the hot path for local feedback.
3. Scroll lists, show dialogs, and draw icons/charts without dropping to a foreign toolkit.
4. Expose an accessibility tree derived from primitive roles.
5. Keep confirmation chrome broker-owned and unforgeable.

Items 1–4 are substantially P0–P2 on the boot spine. Item 5 stays policy-broker owned. Remaining depth: AT-SPI D-Bus, IME, xdg-shell, video decode, full ASL mixins.

## Docs vs code honesty rule

`docs/components/ui-runtime.md` and design-system specs may describe the target language. This maturity matrix plus `03-docs-code-honesty.md` are authoritative for **what boots today**. When a row moves from S/— to P/F, update both files in the same PR as the code.
