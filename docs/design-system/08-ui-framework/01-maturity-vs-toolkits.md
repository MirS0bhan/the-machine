# Maturity vs WinForms / Compose / GTK / Qt

Legend for The Machine column: **F** full · **P** partial in Rust boot path · **S** specified/docs or Python-only · **—** absent.

| Capability | WinForms | Compose | GTK | Qt | The Machine (boot) | Evidence |
|---|---|---|---|---|---|---|
| Widget catalog | F | F | F | F | **P** | Painted: `text`/`field`/`button` (+ confirmation). Grammar also has `stack`/`grid`/`list`/`toggle`/`slider`/`icon`/`media`/`chart`/`dialog` — most unpainted in Rust |
| Layout | F | F | F | F | **P** | `ui-runtime/src/layout.rs` stack v/h + gap + center; `grid` is a tag alias, not a grid algorithm |
| Styling / themes | F | F | F | F | **P** | Dark tokens in `tokens.rs`/`chrome.rs`; `ui.theme.*`; ASL parser is Python-only (`ui-engine/asl_parser.py`) |
| Text / fonts | F | F | F | F | **P→F** | HarfRust shape + FreeType raster (`compositor/src/text.rs`), Inter + JetBrains Mono; bitmap fallback only if faces missing |
| Input events | F | F | F | F | **P** | Pointer click hit-test; keyboard path being added (`system-daemon` → `compositor.input` → `ui.event`) |
| Focus | F | F | F | F | **P** | `compositor.focus` + field focus border; tree-owned tab order in `ui-runtime/src/focus.rs` |
| Accessibility | F | F | F | F | **S** | Roles/labels normative in `01-hig/02-accessibility.md`; no AT-SPI bridge yet |
| Animation | P–F | F | F | F | **S** | Motion curves in ASL docs; no present-loop tweens |
| Drawing / canvas | F | F | F | F | **P** | Rounded rects + glyph blit; no path/clip-stack/images/charts |
| Data binding | F | F | P–F | F | **P** | Patch + `mcp:` bindings; `state:` still mostly read; two-way field edit landing |
| Dialogs / modals | F | F | F | F | **P** | Broker confirmation exclusivity (`elev=e4`); general `dialog` primitive not painted |
| Scrolling | F | F | F | F | **—→P** | Clip helpers + scroll offset props (roadmap P1) |
| Text editing | F | F | F | F | **—→P** | `input_edit.rs` caret/insert/delete on focused field |
| Clipboard | F | F | F | F | **—** | Not implemented |
| DnD | F | F | F | F | **—** | Event vocab only |
| i18n / RTL / IME | F | F | F | F | **—** | HarfRust enables shaping; no locale catalogs / bidi policy yet |
| Windowing | F | F | F | F | **P** | Flat surface map + z-order; partial Wayland SHM; no xdg-shell (G17) |

## What "framework-complete" means here

A toolkit-level Machine UI runtime can:

1. Author every screen with the twelve AUIL primitives + ASL mixins (no ad-hoc paint).
2. Handle pointer **and** keyboard, with focus order and text editing, without an agent in the hot path for local feedback.
3. Scroll lists, show dialogs, and draw icons/charts without dropping to a foreign toolkit.
4. Expose an accessibility tree derived from primitive roles.
5. Keep confirmation chrome broker-owned and unforgeable.

Items 1–3 are P0/P1. Items 4–5 stay P2 / policy-broker owned respectively.

## Docs vs code honesty rule

`docs/components/ui-runtime.md` and design-system specs may describe the target language. This maturity matrix is authoritative for **what boots today**. When a row moves from S/— to P/F, update this table in the same PR as the code.
