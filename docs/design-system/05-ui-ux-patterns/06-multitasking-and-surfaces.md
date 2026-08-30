# Multitasking and Surfaces

Multiple surfaces (`01-hig/03-content-and-voice.md` §3) can exist at once, each independently addressable in the compositor's own model (`docs/components/compositor.md`: `id`, `parent`, `children`, `geometry`, `z_order`, `opacity`, `blurred`, `kind`, `focused`, `label`). This document is the visual language for that — how surfaces relate to each other, not how any one of them lays out its own content (`05-ui-ux-patterns/01-navigation-and-layout.md`).

## 1. Focus and elevation across surfaces

- The focused surface renders at its authored elevation tier unmodified. Unfocused surfaces do not gain extra elevation or shadow — they lose none of their own visual hierarchy internally — but their overall content SHOULD dim slightly (`opacity.dim`, `02-style/01-design-tokens.md` §2) so the focused surface reads as the one the person is currently acting in, without needing to redraw or re-elevate anything.
- Switching focus between surfaces is a compositor-level, real-time operation (Principle 4) — never gated on an MCP round-trip, regardless of what either surface's content happens to be doing.

## 2. The agent's own presence is not a surface

The agent does not occupy its own dedicated app-like surface the way a task does. Its presence — the canvas and greeting (`05-ui-ux-patterns/04-agent-presence-and-conversation.md` §1), the `SuggestionTray`, the local-only and processing-locus indicators (`05-ui-ux-patterns/04-agent-presence-and-conversation.md` §3, §5) — is either the canvas itself or a persistent, summonable overlay available *from* any surface, not a separate window competing with task surfaces for focus. This is a direct visual consequence of the architecture's own framing (`docs/architecture/philosophy.md` commitment 1): the agent is the interface, not an application running alongside other applications.

## 3. `Sheet` anchoring and choreography

`Sheet` (`03-widgets-and-types/02-component-library.md` §1) is the standard way to show auxiliary content beside a surface's primary content without fully replacing it:

- **Modal `Sheet`** (blocks interaction with the surface behind it): entrance `motion.emphasized` sliding in from its anchored edge, backdrop scrim at `opacity.overlay-scrim`, exit `motion.exit` sliding back out the same edge it entered from (`02-style/06-motion.md` §2's directional-consistency rule).
- **Non-modal `Sheet`** (content behind it stays visible and interactive): no backdrop scrim, `elev=e2` rather than a `dialog`-equivalent `e3`, and the surface's primary content reflows to share space with it rather than being obscured — the two are visually peers, not one blocking the other.
- At `compact` size class, every `Sheet` behaves as modal regardless of its declared modality (`03-widgets-and-types/04-composition-and-responsive-layout.md` §3), because there usually isn't room to show both side by side without either becoming too narrow to use.

## 4. Legacy and external surfaces

An `ExternalSurface` (a legacy, non-AUIL window embedded via the compositor's `XReparentWindow` path, `docs/components/compositor.md` § Legacy App Support) cannot participate in this system's elevation or vibrancy signaling — it has no ASL mixins, no tokens, nothing this document set defines. Rather than attempting to fake elevation or vibrancy behavior it structurally cannot have consistently, an `ExternalSurface`'s host frame renders at a fixed `elev=e1` with a `border.default` outline regardless of focus state, so it visually reads as "a guest, consistently framed" rather than "a native surface that's subtly behaving wrong." This is an honest visual admission of a real limitation (`docs/components/compositor.md`: "XWayland apps are second-class — no vibrancy, no confirmation surface integration"), not an attempt to disguise it.

## 5. Multiple outputs

Multi-output (more than one physical display) behavior — where a `Sheet`, a `Toast`, or the Fallback Shell's frozen view render when more than one output exists — is tracked as an open item at the architecture level (`ARCHITECTURE.md` §7, item 15) and is deliberately out of scope here until that's resolved; nothing in this document should be read as implying single-output-only, but nothing here specifies multi-output placement rules yet either.

---

*Cross-references: `01-hig/01-design-principles.md` (Principle 4), `02-style/01-design-tokens.md` §2 (`opacity.dim`), `02-style/05-materials-and-elevation.md` (elevation tiers `Sheet` and `ExternalSurface` use), `02-style/06-motion.md` §2 (directional consistency), `03-widgets-and-types/02-component-library.md` §1 (`Sheet`), `03-widgets-and-types/04-composition-and-responsive-layout.md` §3 (`Sheet` at `compact`), `docs/components/compositor.md` (the real surface model and legacy-app constraints this document gives a visual treatment to), `docs/architecture/philosophy.md` commitment 1.*
