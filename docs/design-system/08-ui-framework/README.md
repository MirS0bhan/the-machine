# UI Framework Maturity

**Fills:** the gap between design-system visual language (`docs/design-system/`) and a production UI toolkit at the level of WinForms, Jetpack Compose, GTK, or Qt.
**Related:** `docs/components/ui-runtime.md`, `docs/components/compositor.md`, `ui-engine/`, `ui-runtime/`, `compositor/`
**Status:** Living gap analysis + implementation contract for the Rust boot path

---

The Machine's UI stack is **agent-native**: an AUIL state tree patched over MCP, styled with ASL tokens, painted by a software compositor. That architecture is intentional — it is not an unfinished clone of Qt. This folder records what that choice still lacks relative to established toolkits, and what the boot path must implement to become a real application UI framework rather than a SessionGreeting shell.

## Contents

| Doc | Purpose |
|---|---|
| `01-maturity-vs-toolkits.md` | Capability matrix vs WinForms / Compose / GTK / Qt, grounded in what the Rust code actually does today |
| `02-implementation-roadmap.md` | P0–P2 work items mapped to concrete modules under `ui-runtime/` and `compositor/` |

## Architectural spine (do not abandon)

```
evdev / key / pointer
        ↓
system-daemon ──provenance──→ compositor.input (hit-test / focus)
        ↓ widget_id
ui.event / ui.patch  ←── agent / lambdas (MCP)
        ↓
ui-runtime layout + ASL tokens
        ↓
compositor.surface + present (HarfRust text, rounded chrome, DRM/fb)
```

Closing toolkit gaps means **growing this spine**, not replacing it with GTK embedding for first-party UI.
