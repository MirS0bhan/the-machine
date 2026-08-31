# Docs ↔ Code Honesty Audit

**Purpose:** Keep design-system / component docs honest about what the Rust boot path implements today versus the normative target language.
**Authoritative for boot reality:** this file + `01-maturity-vs-toolkits.md`. Chapters 01–07 of the design system remain the **normative target**.
**Last audited:** after G17/AT-SPI/IME/video landing.

---

## Reading rules

| Source | How to read it |
|---|---|
| `docs/design-system/01–07` | Normative design target (MUST/SHOULD for *new* UI work). Not a claim that Rust boots it all. |
| `docs/design-system/08-ui-framework/` | Boot-path truth: F / P / S / — matrix + roadmap. |
| `docs/components/ui-runtime.md` | Architecture + **Boot path (today)** MCP/API section. Older NodeKind tables are target/historical. |
| `docs/components/compositor.md` | Pixel + MCP reality first; wlroots/XWayland/GPU blur are **aspirational** unless marked landed. |
| `docs/components/fallback-shell.md` | Recovery *target*; current binary is MCP + console, not a frozen compositor takeover. |
| `ui-engine/` | Python reference grammar (11 primitives historically; design adds `dialog` as 12th). Not the boot daemon. |

---

## Code → docs mismatches (fixed or tracked)

| Finding | Severity | Resolution |
|---|---|---|
| Component docs listed `ui.get_tree` / `ui.get_node`; Rust has `ui.tree` / `ui.get` | High | Boot-path MCP table in `docs/components/ui-runtime.md` |
| Compositor architecture diagram showed wlroots + XWayland + GPU blur as present | High | Diagram + goals rewritten; aspirational called out |
| Design-system README “nothing aspirational” vs 08 matrix S/— rows | Medium | README points readers to 08 for boot reality |
| `ui.auil.parse` / `ui.auil.load` handled but not mcp-bus registered | Medium | Registered |
| Tab focus updated tree but skipped `compositor.focus` | Medium | Synced on Tab |
| Field `caret` prop existed; caret glyph not painted | Medium | Painted when focused |
| `hovered` styled; never set by input | Medium | Pointer `move` → hover props |
| Slider painted; click did not set `value` | Medium | Click/press maps x → value |
| Escape did nothing for dialog | Low | Escape clears soft dialog exclusivity + removes dialog node when present |
| Icon measure/style only; no paint | Low | Geometric glyph paint (no bitmap assets yet) |
| `grid` docs describe 2-axis layout; Rust aliases stack | Documented | Real `grid.rs` landed in P2 |
| xdg-shell absent (G17) | Medium | `xdg_wm_base` v5 via wayland-protocols (not wlroots) |
| No AT-SPI D-Bus | Medium | `org.themachine.A11y` session-bus bridge |
| No IME / locale catalogs | Medium | Compose/dead-key IME + `assets/locales` + `ui.i18n.*` |
| Media procedural only | Low | ffmpeg CLI first-frame RGB decode when available |

---

## Docs → code mismatches (target language still ahead)

These are **expected**; do not “fix” design docs by deleting them — keep 08 honest instead.

| Target claim (01–07 / components) | Boot today |
|---|---|
| Full ASL mixins (`Hoverable`, `Pressable`, …) + motion curves | Ad-hoc press/hover props; opacity tween only |
| Full AT-SPI registry / live regions / refuse empty labels | Machine D-Bus interface + MCP tree; best-effort `org.a11y.Bus` |
| Icon bitmaps / continuous video playback | Geometric icon; first-frame still via ffmpeg |
| Full OS IME buses (ibus/fcitx) | Compose/dead-key only |
| XWayland / wlroots embedding | Not present (xdg-shell without wlroots) |
| Fallback shell frozen AUIL + Ctrl+Alt+F9 takeover | Console / MCP stub |

---

## Boot MCP surface (canonical)

### ui-runtime

`ui.patch`, `ui.get`, `ui.tree`, `ui.bind`, `ui.event`, `ui.status`,  
`ui.focus.get`, `ui.focus.set`, `ui.focus.next`,  
`ui.theme.get`, `ui.theme.set`,  
`ui.auil.parse`, `ui.auil.load`,  
`ui.a11y.tree`, `ui.atspi.status`,  
`ui.i18n.status`, `ui.i18n.t`, `ui.i18n.load`,  
`ui.components.list`

### compositor

`compositor.surface` (`create`/`update`/`destroy`/`geometry`),  
`compositor.focus`, `compositor.input`, `compositor.present`,  
`compositor.list`, `compositor.status` (`xdg_shell: xdg_wm_base.v5`, `video: ffmpeg-cli|procedural`),  
`compositor.blur`, `compositor.confirmation.set_active`

### system-daemon (UI-adjacent)

`clipboard.get`, `clipboard.set` (memory + OS best-effort)  
(+ power / display / net / audio ops — see `docs/components/system-daemon.md`)

---

## Painted AUIL kinds (boot)

| Kind | Layout | Paint | Interaction |
|---|---|---|---|
| `stack` / `container` | yes (+ RTL / locale mirror) | n/a | — |
| `grid` | real cols/span/RTL | n/a | — |
| `text` | yes | label (`i18n:` resolved) | — |
| `field` / `input` | yes | plate + caret | edit, IME compose, clipboard, focus |
| `button` | yes | chrome + press + opacity tween | press/click/Enter |
| `toggle` | yes | track+knob | click flips `checked` |
| `slider` | yes | track+thumb | click sets `value` |
| `list` | yes | rows + clip | wheel scroll |
| `dialog` | leaf card | scrim + card | soft exclusivity; Escape; focus trap |
| `icon` | size tiers | geometric glyph | — |
| `media` | yes | plate + ffmpeg frame or play affordance | focusable; `src`/`url` prop |
| `chart` | yes | axes + bars from `data`/`items` | — |

---

## When to re-run this audit

Any PR that moves an 08 matrix row, adds an MCP method, or paints a new primitive MUST update this file and `01-maturity-vs-toolkits.md` in the same PR.
