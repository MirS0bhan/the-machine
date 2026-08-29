# UI Engine — Declarative Renderer, AUIL/ASL Parser & Patch Protocol

**Fills:** §3.6.2 of `docs/spec.md` (Declarative UI Runtime)  
**Related:** `state-store-spec.md` (UI State Tree persistence), `agent-core-spec.md` (ui.patch tool)  
**Version:** 0.1  
**Status:** Implemented

---

## Overview

The UI Engine is the L5 declarative renderer. It consumes the **UI State Tree** from the State Store and draws it through a pluggable `AbstractRenderer`.

## Modules

| Module | Responsibility |
|--------|----------------|
| `auil_parser.py` | Parse AUIL (Agent UI Layout) into `UINode` trees |
| `asl_parser.py` | Parse ASL state bindings |
| `patch_protocol.py` | Parse and apply patch operations (`~`, `+`, `-`, `!`, `@`) |
| `runtime.py` | `UIRuntime` — holds the live UI State Tree |
| `renderer.py` | `TreeRenderer` + `AbstractRenderer` interface |
| `models.py` | `UINode`, `UIStateTree`, `PatchOperation` |
| `components.py` | Primitive component definitions |
| `mcp_interface.py` | MCP tools: `ui.patch`, `ui.get`, `ui.bind` |
| `server.py` | HTTP + MCP server entry point |

## AUIL example

```
stack#root dir=v gap=m
  text(role=title) "Hello World"
  button#ok label=OK on:press=mcp:app.confirm
```

## Patch protocol

```
~footer(color=accent)
+footer/append: text(role=caption) "Copyright"
-old-banner
```

## Tests

```bash
cd ui-engine && python3 -m pytest test_engine.py
cd ui-engine-demo && python3 -m pytest test_demo.py
```
