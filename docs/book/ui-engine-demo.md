# UI Engine Demo — Pure-Wayland Vertical Slice

**Status:** Implemented. A runnable end-to-end vertical that exercises the full UI
Engine stack (AUIL parse → patch → render → input) without a real GPU.

`ui-engine-demo/` is the reference application that proves the UI Engine design is not
just a spec. Because `wlroots` is a C library (not pip-installable) and this repository
is a logic/architecture project, the demo ships a **terminal-based renderer** that
implements the UI Engine's `AbstractRenderer` interface. The same runtime code that
drives the terminal demo is what a future `wlroots` compositor backend would drive —
only the renderer implementation changes.

---

## 1. Files

| File | Role |
|---|---|
| `demo.auil` | AUIL layout: a root `stack`, a title `text`, an input `field`, a submit `button`, and an output `text`. |
| `wayland_renderer.py` | `WaylandRenderer` — a terminal implementation of `AbstractRenderer` (`create_surface`, `update_surface`, `destroy_surface`, `commit_batch`, `flush`, `get_surface_state`). |
| `demo.py` | App entry point. Loads `demo.auil`, drives the `Runtime`, and runs a raw-mode keyboard input loop (via `tty`/`termios`) that feeds events into the patch pipeline. |
| `test_demo.py` | 20 tests covering parser, renderer, runtime, and the patch→render→output flow. |

---

## 2. The flow

1. `demo.auil` is parsed by `ui_engine.auil_parser` into a node tree.
2. The tree is handed to `ui_engine.runtime.Runtime`, which holds the live UI State Tree.
3. User keystrokes arrive through the input loop and are translated into **patches**
   (`ui_engine.patch_protocol`): `~id(props)` to update text, `+path node` to insert,
   `-id` to remove.
4. Patches are applied to the State Tree and pushed to the `WaylandRenderer`, which
   repaints the terminal.
5. A submit action reads the input field and writes the result into the output label —
   demonstrating input → state → output with no LLM in the real-time path.

---

## 3. Why a terminal renderer

The parent architecture commits to *real-time paths never touching inference*. The
terminal renderer honors that: it is fully deterministic and depends only on the
`AbstractRenderer` contract. Swapping in a `wlroots` surface later requires no changes
to the parser, patch protocol, or runtime — only a new `AbstractRenderer`
implementation. This is the practical expression of the UI Engine's "pluggable
renderer" design.

---

## 4. Known integration seam

The demo's patch tests exercise `ui_engine.patch_protocol` directly. That module is the
shared contract between the Agent Core's emitted patches and the UI Runtime's
application step, so any bug there surfaces first in `test_demo.py`. It is the natural
place to extend when wiring the Agent Core's `ui.patch` tool to a live surface.

---

## 5. Running it

```bash
cd ui-engine-demo
uv run python demo.py          # interactive terminal app
uv run pytest test_demo.py -q   # test suite
```

[TOC]
