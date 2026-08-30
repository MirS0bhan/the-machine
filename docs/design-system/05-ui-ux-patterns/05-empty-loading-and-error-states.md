# Empty, Loading, and Error States

These are the states a screen is in *before* it has the content it's ultimately for — and, distinctly, the state the whole system is in when something below the screen itself has gone wrong. This document also draws the line between "the agent is still composing a normal screen" and "the agent isn't the one rendering this screen at all," which is the Fallback Shell's territory (§4).

## 1. Empty states

Not all emptiness means the same thing, and `EmptyState` (`03-widgets-and-types/02-component-library.md` §6) is composed differently for each:

| Kind | Content | Action |
|---|---|---|
| **First-run** (nothing has ever been added) | An explanation of what would normally appear here | The specific action that would create the first item, named directly ("Add your first device"), not a generic "Get started" |
| **Empty after filtering/search** | A statement that the filter/search produced nothing, not that the underlying collection is empty | An action to clear the filter/search, not to create new content |
| **Genuinely nothing to show yet, pending an async result** | This is `loading` (§2), not an empty state — don't show "No items" for half a second before real content arrives |

An `EmptyState`'s `icon` slot is decorative and MAY be omitted; its `title` is not, and MUST distinguish which of the two real kinds above it is (never a bare "Nothing here" that leaves the person guessing whether that's because they haven't done anything yet or because their search matched nothing).

## 2. Loading: skeleton vs. spinner

| Situation | Use |
|---|---|
| The content's eventual shape is already known (a list of a known row type, a card layout) and the wait is likely to be noticeable | `Skeleton` (`03-widgets-and-types/02-component-library.md` §6) — placeholder shapes matching the real content's layout, so the transition from skeleton to real content doesn't cause a layout jump |
| The content's shape is not yet known, or the wait is expected to be brief | `Spinner` |
| A specific, already-visible control is waiting on its own bound intent | That primitive's own `state:loading` treatment (`03-widgets-and-types/03-states-and-variants.md` §2) — no separate full-region loading indicator layered on top of a single control that already has one |

A `Skeleton`'s shimmer motion is continuous and decorative — it MUST NOT be the thing communicating *that* loading is happening to assistive technology (that's the `announce=polite` on entering `state:loading` itself, `01-hig/02-accessibility.md` §8); the shimmer is a sighted-user perceptual aid only.

## 3. Error states

| Scope | Component |
|---|---|
| One field or control failed validation or its own action | Inline status (`05-ui-ux-patterns/02-feedback-and-status.md` §4) |
| One region's content failed to load, but the rest of the surface is fine | A localized error treatment reusing `EmptyState`'s anatomy — icon, `status.destructive`-colored, a specific message, and (where meaningful) a retry action in place of the "create" action a true empty state would offer |
| The entire surface failed to do the one thing it exists for | `Banner` at the top of the surface, or (if nothing on the surface is usable at all) a full-surface error treatment — still agent-composed, still using ordinary tokens, still nothing like the Fallback Shell's appearance (§4) |

Retrying is always an explicit action the person takes, never an automatic silent retry loop that could mask a real, ongoing problem — if the system retries automatically at the infrastructure level, the UI still only updates when there's something new to report, per the routine-vs-new-information distinction in `01-hig/02-accessibility.md` §8.

## 4. Degraded and offline states are not one thing

This system has two entirely different "something's wrong" tiers, and conflating them in this document would be inaccurate:

1. **The agent is fine; something it depends on isn't** (the cloud tier is unreachable, a specific lambda crashed). The agent is still composing ordinary, agent-authored UI using this document set's ordinary tokens and components — most commonly a `Banner` ("Couldn't reach the cloud model — continuing with what's available locally," per `01-hig/03-content-and-voice.md` §4's structure) or a task-specific error per §3. Nothing about this state uses a different visual system.
2. **The agent itself is unavailable** (crashed, still cold-booting, unreachable). At that point the UI Runtime is rendering the last known-good UI State Tree, frozen and read-only, with the Fallback Shell's own fixed "Agent Unavailable" banner and recovery console (`docs/components/fallback-shell.md`) — which, per `02-style/05-materials-and-elevation.md` §5, uses its own independent, fixed palette and is **not** an instance of anything in this document set. Nothing in `03-widgets-and-types/` or `02-style/` is rendered by the Fallback Shell, and nothing the Fallback Shell renders should be copied as an example of this system's tokens.

The practical test for which tier applies: if `agent.status()` (`docs/components/agent-core.md`) would report anything other than fully offline, tier 1 applies and this document's ordinary components are correct. If the Agent Core itself is the thing that's down, tier 2 applies and this document set has nothing more to say — that's the Fallback Shell's document, not this one.

---

*Cross-references: `01-hig/02-accessibility.md` §8 (announcement rules for loading/error transitions), `01-hig/03-content-and-voice.md` §4 (empty/error copy), `02-style/05-materials-and-elevation.md` §5 (why the Fallback Shell is outside this token system entirely), `03-widgets-and-types/02-component-library.md` §6 (`EmptyState`/`Skeleton`), `03-widgets-and-types/03-states-and-variants.md` §2 (`state:loading`/`state:error`), `05-ui-ux-patterns/02-feedback-and-status.md` (transient/persistent feedback, as distinct from the states in this document), `docs/components/fallback-shell.md`, `docs/components/agent-core.md`.*
