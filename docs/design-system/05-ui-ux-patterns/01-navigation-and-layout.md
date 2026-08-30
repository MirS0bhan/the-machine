# Navigation and Layout

A **surface** (`01-hig/03-content-and-voice.md` §3) is this system's unit of navigable UI — the canonical term for what a compositor-level `Surface` (`docs/components/compositor.md`) actually renders, addressed by its own `ui.<tree>` in the State Store (`docs/architecture/layers.md` §1.2). This document is about navigating *within* one surface; navigating *among* several concurrent surfaces is `05-ui-ux-patterns/06-multitasking-and-surfaces.md`.

## 1. Surface anatomy

Most surfaces share the same top-level shape, composed from `03-widgets-and-types/02-component-library.md`:

```
stack#root dir=v
  AppBar               — leading/title/actions chrome (§1's header)
  stack dir=h          — only if the surface has persistent side navigation
    NavList            — present only when there's more than one top-level destination
    stack              — the body: everything else in this document
```

A surface with only one logical destination (a single-purpose utility) omits `NavList` entirely — Principle 1 applies to navigation chrome exactly as it applies to any other structure: a nav rail with one destination in it is not navigation, it's decoration.

## 2. Navigation patterns

| Pattern | Component | When |
|---|---|---|
| **Peer switching** | `TabBar` | A small, fixed number of equally-important views of the same underlying content |
| **Persistent destinations** | `NavList` | A surface with several genuinely distinct sections a person returns to repeatedly |
| **Drill-in / hierarchical** | An item `press` pushes a new view (§3) | Browsing from a list into an item's detail |
| **Local, scoped switching** | `SegmentedControl` | Switching a filter/view *within* one region, not the whole surface |
| **Path awareness** | `Breadcrumb` | Only once drill-in depth exceeds two levels — below that, a `Breadcrumb` adds chrome without adding orientation (Principle 1) |

## 3. Drill-in, back, and forward are local and deterministic

Moving into and back out of an already-visited view MUST NOT require waking the Agent Core. The UI Runtime maintains a lightweight per-surface navigation stack (a sequence of view checkpoints, not full UI State Tree snapshots) and resolves `mcp:nav.back` / `mcp:nav.forward` through a fixed, always-registered system-level handler — the same category of thing as `ui.status`, never a candidate for agent reasoning. This matters for a reason beyond speed: navigating backward is exactly the kind of task that should "retire" (`docs/architecture/philosophy.md` commitment 5) the instant it's ever been done once, and there's no reason to re-derive "what was on screen before" through reasoning when the runtime already has it.

A drill-in transition (list → detail) uses `motion.standard` (`02-style/06-motion.md`); its reverse (detail → list, via back) uses `motion.exit`, following the entrance/exit asymmetry rule in `02-style/06-motion.md` §2.

## 4. The canvas

The root of everything — the surface (or surfaces) rendered before any specific task's UI exists — is the **canvas**. It is where `SessionGreeting` (`05-ui-ux-patterns/04-agent-presence-and-conversation.md` §1) first renders, and where a `SuggestionTray` may persist across tasks. The canvas is not itself a "surface with content" in the §1 sense — it has no `AppBar`, no drill-in stack, and no back/forward history of its own, because it isn't a task, it's the resting state between tasks.

## 5. Landmarks

Every surface's top-level regions (`AppBar`, `NavList`, the body) SHOULD carry a `label=` even where no sighted person would need one, because assistive technology's landmark navigation depends on distinguishing them — a surface with an unlabeled `AppBar` and an unlabeled body region is navigable by sight and considerably harder to navigate by landmark-jumping alone.

---

*Cross-references: `01-hig/01-design-principles.md` (Principle 1, applied to nav chrome), `01-hig/03-content-and-voice.md` §3 ("surface," the canonical term), `02-style/06-motion.md` §2 (entrance/exit asymmetry), `03-widgets-and-types/02-component-library.md` §2 (`AppBar`/`NavList`/`TabBar`/`SegmentedControl`/`Breadcrumb`), `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §1 (the canvas's greeting), `05-ui-ux-patterns/06-multitasking-and-surfaces.md` (navigation *among* surfaces).*
