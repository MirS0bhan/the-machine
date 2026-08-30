# Feedback and Status

This document is the decision tree behind every "something happened, tell the person" moment, and the components in `03-widgets-and-types/02-component-library.md` §6 that implement each branch of it.

## 1. Choosing the right surface for a message

Three questions, in order, pick the component:

1. **Does it require a decision before anything else can happen?** → Yes: `AlertDialog` (acknowledge-only) or `ConfirmDialog` (a real choice), `03-widgets-and-types/02-component-library.md` §7. No: continue.
2. **Is it about a specific piece of content (a field, a row, a card) or about the surface/task as a whole?** → Specific: inline status (§3, below). Whole-surface: continue.
3. **Does the person need to be able to act on it, or read it later, or does it just need to be seen once?** → Needs an available action or persists until dismissed: `Banner`. Transient, no action needed beyond acknowledgment: `Toast`.

| | Blocking | Persistent | Transient |
|---|---|---|---|
| **Whole-surface** | `AlertDialog` / `ConfirmDialog` | `Banner` | `Toast` |
| **Specific content** | (rare — an inline validation blocking submit still isn't a dialog, see `03-forms-and-data-entry.md` §3) | Inline status / `Tag` | A field's transient helper text |

## 2. `Toast`

- Auto-dismisses after a fixed interval (~4s for a one-clause message, extended proportionally if it carries a single action) or on an explicit dismiss.
- Multiple toasts stack, most recent nearest the anchor edge, oldest pushed out and dismissed early if more than three would be visible at once — a toast queue is not a notification history; anything worth keeping belongs in a persisted list, not a stack of transient toasts.
- `announce=polite` always (`01-hig/02-accessibility.md` §8) — never `assertive`, because by definition nothing routed to a `Toast` was judged important enough to block on.
- Entrance `motion.gentle`, exit `motion.exit` (`02-style/06-motion.md`), sliding from the edge it's anchored to, consistent with §2's directional-consistency rule.

## 3. `Banner`

- Persists until dismissed or until its underlying condition resolves (a `Banner` about a lost network connection clears itself the instant connectivity returns — it does not wait for the person to notice and dismiss it manually).
- Carries `severity=` (`info` | `positive` | `warning` | `destructive`, matching `status.*` tokens) which sets both its color and its `announce=` default: `info`/`positive` → `polite`; `warning`/`destructive` → `assertive`.
- MAY carry actions (`Banner`'s `actions` slot) — this is the main thing that distinguishes it from a `Toast` besides persistence.

## 4. Inline status

- A `field`'s validation message (`03-forms-and-data-entry.md` §3) and a `list` row's status `Tag` are both inline — they render in-place, next to the thing they describe, rather than floating over the surface.
- Inline status follows the same redundant-coding rule as everything else (§5) but does not need its own `announce=` region distinct from the field/row it's attached to — the field's own `state:error` transition already carries the announcement.

## 5. Redundant status coding (never color alone)

Per Principle 7 and `01-hig/02-accessibility.md` §3, every `status.*` token pairs with a fixed icon, and the icon and the color are never optional relative to each other:

| Token | Required icon | Typical text framing |
|---|---|---|
| `status.positive` | A check-circle glyph | "Saved," "Connected," "Completed" |
| `status.warning` | A triangle-exclamation glyph | "Check before continuing," a non-blocking heads-up |
| `status.destructive` | A circle-x or alert-octagon glyph | "Failed," "Couldn't connect," any error per `01-hig/03-content-and-voice.md` §4 |
| `status.info` | An info-circle glyph | Neutral, non-urgent context |

A component that renders a `status.*` background or text color without its paired icon is a defect at the same tier as a missing accessible label — this isn't a stylistic nicety, it's what makes status legible to color vision deficiency and to a black-and-white or high-contrast render alike.

## 6. Progress

| Situation | Component |
|---|---|
| A known-duration or known-fraction operation happening because of something the person is looking at right now | Determinate `slider(interactive=false)`, inline |
| An operation of unknown duration, brief enough that naming it would be overkill | `Spinner` |
| A background task (a lambda invocation, a longer download) the person may check on later, away from where they started it | `ProgressCard` (`03-widgets-and-types/02-component-library.md` §5) |

A progress indicator's value updates are routine content patches — silent per `01-hig/02-accessibility.md` §8 — right up until the task *completes* or *fails*, at which point that specific transition (not the ticking itself) is what gets `announce=polite`/`assertive` and, if the person isn't looking at the `ProgressCard` anymore, a `Toast` or `Banner` surfacing the same completion.

---

*Cross-references: `01-hig/01-design-principles.md` (Principle 7), `01-hig/02-accessibility.md` §3, §8 (redundant coding, announcements), `01-hig/03-content-and-voice.md` §4 (message copy structure), `02-style/02-color-and-surfaces.md` §4 (`status.*` tokens), `03-widgets-and-types/02-component-library.md` §6–§7 (`Toast`/`Banner`/`ProgressCard`/dialog family), `05-ui-ux-patterns/05-empty-loading-and-error-states.md` (full-surface error/loading, as opposed to the transient/inline feedback here).*
