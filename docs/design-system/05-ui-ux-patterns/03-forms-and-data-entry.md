# Forms and Data Entry

Forms are where accessibility, content, and state rules (`01-hig/02-accessibility.md`, `01-hig/03-content-and-voice.md`, `03-widgets-and-types/03-states-and-variants.md`) all apply to the same few square inches of screen at once. This document is how they compose in practice.

## 1. Labels are always visible

A `field`'s label MUST be a real, persistently visible `text(role=label)` node, occupying `Field`'s `label` slot — never simulated with `placeholder=` text that disappears the moment the person starts typing. This is not merely an `01-hig/02-accessibility.md` §1 labeling requirement; it's a usability one independent of accessibility — a person who pauses mid-form and returns to a field with typed content and no visible label has no way to recall what that field was for.

`placeholder=` MAY still be used, but only for a genuine example of the expected format ("+1 555 555 5555"), never for the field's identity.

## 2. Grouping

- Use `FieldGroup` (`03-widgets-and-types/02-component-library.md` §4) to bind a label, its field, and any help/error text into one unit that moves, hides, and validates together.
- Related fields (a full name split into first/last, an address's city/region/postal code) group visually with a tighter `gap` (`s-sm`) than the `gap` between unrelated field groups (`s-xl`) — the spacing itself communicates the grouping, on top of any explicit section heading.
- Adapts across size classes per `03-widgets-and-types/04-composition-and-responsive-layout.md` §3 — single column at `compact`, optionally multi-column for genuinely related short fields at `standard`/`expansive`.

## 3. Validation

- **Timing:** validate on blur by default (`on:blur` triggers the check, not a live keystroke-by-keystroke check) except for a field with a hard format constraint the person benefits from seeing corrected immediately (a numeric-only field silently rejecting a letter as it's typed, rather than accepting it and complaining after the fact) — that distinction is the actual rule, not "live validation is always better" or "always wait for blur."
- **Cross-field validation** (password confirmation matching, a date range's end preceding its start) resolves on submit, since it's meaningless before both fields have values.
- **Placement:** an error message renders directly below its field, inside the `FieldGroup`'s `help` slot, replacing any neutral help text that was there — never in a separate, disconnected error summary alone (a summary MAY additionally exist at the top of a long form, linking down to each error, but never as the *only* place the error appears).
- **Rendering:** `state:error` (`03-widgets-and-types/03-states-and-variants.md` §2) swaps the field's border to `status.destructive` and shows the paired icon+text per `05-ui-ux-patterns/02-feedback-and-status.md` §5 — color alone never carries the error.
- **Copy:** follows `01-hig/03-content-and-voice.md` §4's three-part structure, and specifically avoids blaming phrasing ("Enter a value between 1 and 100," not "Invalid value").

## 4. Required and optional marking

Mark **optional** fields explicitly (a `(optional)` suffix appended to the label, in `text.tertiary`), and leave required fields unmarked. Most real forms in this system have more required fields than optional ones; marking the minority is less visual noise than marking the majority; and — unlike a bare asterisk — the word "optional" needs no legend to be understood.

## 5. Multi-step forms

- A wizard's current position renders as a determinate `slider(interactive=false)` or a small fixed step-indicator row (a `list(dir=h)` of `Tag`-like step markers, one `state:selected` for the current step) — never invented as a one-off, always one of these two already-defined component compositions.
- Back moves within a multi-step form the same way surface-level back does (`05-ui-ux-patterns/01-navigation-and-layout.md` §3) — local, deterministic, no agent round-trip, and it MUST preserve every already-entered value.
- Abandoning a multi-step form with unsaved input triggers a `ConfirmDialog` ("Discard your changes?"), never a silent discard and never the Broker-owned Confirmation Surface (§4's distinction in `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §2 — this is an ordinary, agent-renderable decision, not a capability grant).

## 6. Voice-driven form filling

Per `04-events/04-multimodal-and-voice-input.md`, any `field` with `input-mode=voice`/`hybrid` fills the same way a typed value would — an utterance resolves to a `change` event and the same validation in §3 applies to the result. A form that accepts voice input for one field and not another within the same `FieldGroup` sequence is a worse experience than accepting it consistently or not at all; input-mode should be a property of the *form's* design intent, not decided field-by-field.

## 7. Destructive submission

A submit action that is itself destructive or irreversible (deleting an account, overwriting existing data) is not "just another submit" — it follows the destructive-action content rule in `01-hig/03-content-and-voice.md` §4 (name the specific destroyed object) and, if the action would also touch a protected capability, routes through the Broker's Confirmation Surface exactly as any other protected request would, not through the form's own submit button styled to look serious.

---

*Cross-references: `01-hig/02-accessibility.md` §1 (label requirements), `01-hig/03-content-and-voice.md` §4 (error and destructive-action copy), `02-style/01-design-tokens.md` §3 (the `space` tiers used for field grouping), `03-widgets-and-types/02-component-library.md` §4 (`Field`/`SearchField`/`CheckboxRow`/`FieldGroup`), `04-events/04-multimodal-and-voice-input.md` (voice form-filling), `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §2 (why destructive submission is not the Confirmation Surface unless a capability is actually involved).*
