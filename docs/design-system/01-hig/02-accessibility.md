# Accessibility Standards

Accessibility is enforced structurally wherever possible (Principle 5, `01-design-principles.md`) rather than left to review. This document defines the specific, checkable standards that structural enforcement is built against, and the smaller set of things that genuinely do require a human or agent decision rather than a default. It also resolves open item 6 in `ui-engine/README.md` ("Accessibility mapping — semantic roles → AT output") by making the role/label mapping in §1–§2 the canonical one.

## 1. Labeling

Every interactive primitive (`button`, `field`, `slider`, `toggle`, `list` items, `dialog` actions) MUST resolve to a non-empty accessible label at render time. The resolution order:

1. An explicit `label=` prop, if present.
2. The primitive's text child content, if it has one (a `button` wrapping a `text "Save"` child needs no separate label).
3. For icon-only controls (a `button` with only an `icon=` prop and no text child), an explicit `label=` is **mandatory**, not optional — there is no structural fallback for an icon alone, since icon names are not guaranteed to be self-describing to assistive technology.

The UI Runtime MUST refuse to render an interactive node that resolves to an empty label rather than silently rendering it unlabeled. This is a validation failure at the same tier as a malformed patch op, not a soft warning.

## 2. Roles

Every primitive declares a fixed accessible role as part of its type definition (see the role column in `03-widgets-and-types/01-primitive-types.md`) — `button` is always role `button`, `toggle` is always role `switch` (or `checkbox` / `radio` when its `variant=` prop is set, `03-widgets-and-types/01-primitive-types.md` §7), and so on. Composed components (`03-widgets-and-types/02-component-library.md`) inherit their role from their root primitive unless the component definition explicitly overrides it (e.g. a `Row` component built from a `stack` wrapping a `button` reports role `button`, not the generic container role its outer `stack` would otherwise imply).

Custom composite components that don't cleanly map to an existing role MUST declare one explicitly in their `component` definition (`group`, `list`, `region`, etc.) rather than defaulting to no role at all.

## 3. Contrast

- Text contrast against its immediate background MUST meet a 4.5:1 ratio for body-scale text and 3:1 for large-scale text (`title-1` and above, per the type scale in `02-style/03-typography.md`), evaluated **after** any vibrancy/blur is applied (`02-style/05-materials-and-elevation.md`) — a token pair that passes contrast on a flat swatch but fails once blurred is a token bug, not an acceptable trade-off for visual richness.
- This is why every `surface.*` / `text.*` token pair in `02-style/01-design-tokens.md` is defined and tested together, never independently — a text token is only "correct" in the context of a specific surface token, and the token reference notes which pairings are pre-validated.
- Status MUST NOT be communicated by color alone (Principle 7). Every status token (`status.destructive`, `status.warning`, etc.) is paired with an icon or text label requirement in the component that uses it — see `05-ui-ux-patterns/02-feedback-and-status.md`.

## 4. High contrast mode

High contrast is a first-class render state, not an optional theme. Every opacity-based token (`opacity.border`, `opacity.dim`, `opacity.disabled` in `02-style/01-design-tokens.md`) carries an explicit high-contrast variant that the UI Runtime applies automatically when the system preference is active — no component or agent-authored patch needs to branch on this state manually. Vibrancy/blur materials fall back to a bordered, non-blurred treatment in high-contrast mode (`02-style/05-materials-and-elevation.md` §4).

## 5. Reduced motion and reduced transparency

Both are system-level preferences the UI Runtime resolves automatically, the same way `adaptive(light:.. dark:..)` resolves automatically for color:

- **Reduced motion:** every `motion=` curve (`02-style/06-motion.md`) collapses to a near-instant crossfade (the fixed `motion.reduced` duration) rather than being disabled outright — state changes must still be perceivable, just not via spring physics. Components MUST NOT hardcode a motion curve in a way that bypasses this substitution.
- **Reduced transparency:** every `vibrancy=` level collapses to its opaque fallback color (already defined per-token as the non-vibrant base value, `02-style/01-design-tokens.md`). No separate "reduced transparency" token set is needed because every vibrant token already carries its own flat fallback as part of its definition.

## 6. Keyboard and focus

- Every interactive primitive MUST be reachable via sequential keyboard navigation in the order it appears in the current UI State Tree, unless a component explicitly declares a different focus order (a `dialog`'s internal tab order, for instance, which is scoped and trapped — see §7).
- Activation (`Enter`/`Space`-equivalent) MUST trigger the same `on:press`/`on:change` intent a pointer activation would — there is no separate keyboard-only code path in AUIL, which is a direct consequence of the event model (`04-events/01-event-model.md`) binding one intent name to one semantic action regardless of input modality.
- Focus state is a motion event (`on:focus =>`, resolved locally per Principle 4) for the visual ring, but focus *movement* between components is state the UI Runtime tracks structurally — it MUST be preserved across a patch (`~`/`+`/`-`) unless the focused node itself was the one removed, per the patch protocol's own scroll/focus-preservation guarantee (`ui-engine/docs/spec.md`, Patch protocol; the underlying state-persistence rationale is `docs/spec.md` §3.2.2).

## 7. Modal focus trapping

The `dialog` primitive (`03-widgets-and-types/01-primitive-types.md` §12) MUST trap keyboard focus within itself while open — Tab/Shift-Tab cycles only through the dialog's own interactive children, and focus returns to whatever triggered the dialog when it closes. This is a primitive-level guarantee, not something each dialog-using component needs to re-implement.

## 8. Live region announcements for agent-driven patches

Because this system's UI updates are frequently agent-initiated rather than user-initiated (a background task completing, a lambda finishing, a proactive status change), assistive technology needs to know when to announce a change and when to stay silent — a screen reader that announces every single `~id(props)` patch would be unusable.

Rule: a patch triggers an announcement only if it targets a node explicitly marked `announce=polite` or `announce=assertive` in its definition (mirroring the urgency distinction between a `Toast`/`Banner` and a routine content update — see `05-ui-ux-patterns/02-feedback-and-status.md`). Routine content patches (a list refreshing, a progress value ticking) are silent by default; state changes that represent new information the person hasn't seen yet (a task finished, an error occurred) MUST be marked and MUST announce.

## 9. Minimum interactive target size

Every interactive primitive's hit target MUST be at least the platform's defined minimum regardless of its visual size — a visually compact icon button still gets its hit area padded out to the minimum invisibly, rather than shrinking the target to match a small icon. The exact minimum is a token (`space.min-target` — see `02-style/01-design-tokens.md`), not a hardcoded value repeated per component, so it can be raised uniformly if the platform's input model changes (e.g. touch added later, per `ARCHITECTURE.md` §7 item 5, "multi-modal input handling").

## 10. What is NOT automatic — decisions that still require a human or agent judgment call

Structural enforcement covers labeling, roles, contrast, motion/transparency substitution, focus order, and target size. It does **not** cover:

- **Whether an icon-only control's label text is actually descriptive** ("Button 3" passes the non-empty check and is still useless).
- **Reading order for genuinely complex custom layouts** where visual order and logical order diverge (e.g. a two-column comparison that should read row-by-row, not column-by-column).
- **Whether a color choice, even a passing-contrast one, is distinguishable enough for the most common forms of color vision deficiency** — contrast ratio and color-blind-safe are related but not identical properties.

These remain real review items. The point of everything above is to shrink the review surface to genuinely judgment-dependent cases, not to eliminate review entirely.

---

*Cross-references: `02-style/01-design-tokens.md` (opacity/contrast/min-target tokens), `03-widgets-and-types/01-primitive-types.md` (per-primitive role/label defaults), `04-events/01-event-model.md` (why keyboard and pointer share one intent path).*
