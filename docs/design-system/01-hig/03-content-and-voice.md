# Content and Voice

Every string the system renders was, at the moment of rendering, either **agent-authored** (composed by Tier A or Tier B into a `text` node or a component prop) or **system-authored** (a fixed string owned by a deterministic, non-LLM component — the Confirmation Surface Daemon's templates, the Fallback Shell, a validation error a primitive generates structurally). This document sets the rules for both, and §7 explains why the distinction is load-bearing, not stylistic.

## 1. Voice

The system's voice is **calm, concrete, and load-bearing** — every word should be doing work the reader needs, not performing warmth or personality on top of it.

1. **MUST** state what happened or what is needed before any elaboration. Lead with the fact, not the framing.
2. **MUST NOT** perform emotion the system doesn't have. No "Oops!", "Whoops!", "Great job!", or exclamation marks used for enthusiasm rather than genuine urgency. A system that apologizes for every error trains the person to distrust its confidence signals elsewhere.
3. **SHOULD** prefer the active voice and a named actor. "The download failed" is weaker than "`video_player` couldn't reach the network" — the second gives the person something to act on.
4. **MUST NOT** blame the person. "Invalid input" is a description of the system's confusion, not the person's error; prefer "Enter a number between 1 and 100" (says what *will* work) over "You entered an invalid number" (says what went wrong, from the wrong point of view).
5. **MAY** be warm in genuinely low-stakes, agent-initiated conversational turns (the greeting in `05-ui-ux-patterns/04-agent-presence-and-conversation.md` is allowed a little more personality than a validation error ever should be) — but warmth is a property of *word choice*, never of adding filler sentences, hedging, or unnecessary acknowledgment ("Sure, I can help with that!") before the actual content.

## 2. Mechanics

| Rule | Value | Applies to |
|---|---|---|
| Capitalization | Sentence case (only the first word and proper nouns capitalized) | Button labels, field labels, headings, menu items, toasts |
| Capitalization | Title Case reserved for proper nouns only | Surface titles that *are* a proper noun (a document's own title, a person's name) — never invented for emphasis |
| Terminal punctuation | None on labels, titles, or single-sentence status text | Buttons, field labels, `caption`/`label` text roles |
| Terminal punctuation | Full sentences end in a period | Body text, error/help text of more than one clause, dialog body copy |
| Exclamation marks | Reserved for genuine urgency communicated nowhere else (rare) | Never for routine success ("Saved!" → "Saved") |
| Ellipsis (`…`) | Only on a label that starts an interaction requiring more input | `"Rename…"` (opens a field), not `"Loading…"` (use a `loading` state per `03-widgets-and-types/03-states-and-variants.md`, not punctuation, to communicate in-progress work) |
| Numerals | Digits, not spelled out, in any UI chrome | "3 items", not "Three items" — spelled-out numbers only occur inside agent-composed prose sentences where a digit would look odd |
| Truncation | Truncate mid-string with a single `…`, never mid-word if a word boundary exists within the available width | A `text` node MUST NOT be hard-clipped without an ellipsis — a silently cut-off string is a content bug, not a layout constraint |

## 3. Canonical terminology

The system's own vocabulary is part of its content, and inconsistent terminology is a content bug the same way inconsistent color would be a style bug. This table is the single source; do not introduce a synonym for anything in the left column.

| Use | Not | Why |
|---|---|---|
| **surface** | window, view, screen (as a noun for a UI container) | `compositor.surface` and the UI State Tree are already surface-addressed (`docs/components/compositor.md`, `docs/components/ui-runtime.md`); "window" implies a manual, human-managed chrome model this system doesn't have |
| **task** | job, action, operation (for something the agent is doing on the person's behalf) | Matches `task.*` State Store namespace and `task-complete` event category |
| **capability** | permission, access, scope | Matches `CAP_*` manifests and Policy Broker terminology exactly |
| **lambda** | function, script, plugin (for a registered, sandboxed handler) | Matches Lambda Server terminology; "plugin" implies user-installed, which is not always true |
| **confirm** / **deny** | approve/reject, yes/no, allow/block (for the Confirmation Surface's two outcomes) | Matches the Policy Broker's own `ALLOW`/`DENY`/`CONFIRM` decision vocabulary; label rotation for anti-automation (`05-ui-ux-patterns/04-agent-presence-and-conversation.md` §3) is the one sanctioned exception |
| **local-only mode** | offline mode, airplane mode, privacy mode | This is a specific, named system setting (`agent.local_only_mode`), not a general description |

## 4. Writing by category

- **Actions (buttons, menu items):** verb-first, object second, no article unless needed for clarity — `"Save"`, `"Delete draft"`, `"Add device"`. A destructive action names the destroyed object explicitly rather than relying on context — `"Delete draft"`, not a bare `"Delete"` sitting next to three other deletable things.
- **Field labels:** a noun phrase describing the *value*, not an instruction — `"Email address"`, not `"Enter your email address"`. Placeholder text (if any) MUST NOT substitute for a label — labels are always visible per `05-ui-ux-patterns/03-forms-and-data-entry.md` §1.
- **Errors (validation, failures):** three parts, in order, and MAY omit the third if the fix is self-evident from the first two: **(1) what happened**, **(2) why, if knowable and useful**, **(3) what to do next**. "Couldn't connect to `video_player` — the network is unreachable. Check your connection and try again." SHOULD NOT surface raw system error strings (stack traces, error codes without translation) to the person; those belong in `shell.status`/logs (`05-ui-ux-patterns/05-empty-loading-and-error-states.md`), not the primary message.
- **Empty states:** say what's missing and, if there's an action that would fill it, name that action — never just "No items" alone when "Add your first item" is available and true.
- **Confirmations (destructive or protected-unit):** the template fields are fixed by the Confirmation Surface Daemon (`docs/components/policy-broker.md` § Confirmation Surface) — `requester`, `description`, `capability`, `scope` — and every one of them MUST be filled with the specific, real values of the actual request. A confirmation that reads "This action cannot be undone" without naming the action is a content failure, not an acceptable generic fallback.
- **Toasts and banners:** one clause. If the message needs a second clause to be useful, it should be a banner with a body, not a toast (`05-ui-ux-patterns/02-feedback-and-status.md` §1).
- **The agent's own conversational turns** (greetings, clarifying questions, narrated status): MAY read as a single coherent voice rather than a list of terse fragments, but MUST still obey §1's no-performed-warmth rule and MUST NOT ask a clarifying question the context already answers (re-asking wastes the one thing this system is supposed to save — the person's attention).

## 5. Numbers, dates, and units

- Dates and times render through the locale-aware formatting path, never hand-assembled (no `"{month}/{day}/{year}"` string concatenation) — this is what makes §2 of `04-inclusive-and-adaptive-design.md` possible without per-string rework later.
- Durations round to the coarsest unit that keeps the value meaningful for its context — a progress readout showing seconds remaining SHOULD switch to minutes past 90 seconds rather than reading "127 seconds left."
- Units are never abbreviated ambiguously — "min" for minutes is fine; invented abbreviations are not.

## 6. Agent-authored vs. system-authored copy

This distinction exists because of a specific security property, not as a stylistic nicety: **the Confirmation Surface's copy is authored by the Broker's fixed templates, never by the agent** (`docs/components/policy-broker.md` § Template System; `docs/architecture/philosophy.md` commitment 10, "Confirmation the Agent Cannot Forge"). A person who has learned to trust the tone and structure of a confirmation prompt is relying on that tone being *unforgeable* — if agent-composed prose could appear inside a confirmation template, a compromised or hallucinating agent could write a misleading description of what it's asking for. Concretely:

- Every placeholder inside a Confirmation Surface template (`requester`, `description`, `capability`, `scope`, `confirm_label`, `timeout`) is filled with a value the Broker itself reads from the request, not with agent-generated prose describing the request.
- Agent-composed `text` nodes elsewhere in the UI tree are free-form within §1–§5's rules, and MAY narrate, summarize, or explain anything the agent is doing — the constraint is specific to the one surface the agent cannot render into at all.

---

*Cross-references: `01-hig/01-design-principles.md` (Principle 5's accessible-by-construction stance, which content correctness feeds into), `01-hig/04-inclusive-and-adaptive-design.md` (localization-ready copy), `05-ui-ux-patterns/02-feedback-and-status.md` (toast/banner copy length), `05-ui-ux-patterns/03-forms-and-data-entry.md` (field labels), `05-ui-ux-patterns/04-agent-presence-and-conversation.md` (confirmation template copy, agent conversational voice).*
