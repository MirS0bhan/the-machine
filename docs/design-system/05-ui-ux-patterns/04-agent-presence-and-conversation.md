# Agent Presence and Conversation

This document is where the agent itself — not just the content it renders — becomes visible: the login greeting, the Confirmation Surface it cannot render into, the quick actions it proposes unprompted, and the small, honest signals that keep its presence legible rather than ambient and untraceable. It resolves the confirmation-UX open item that motivated much of this document set's grounding work and gives `docs/components/policy-broker.md`'s Confirmation Surface mechanics a concrete visual design for the first time.

## 1. The greeting

The first patch a person sees after unlocking is `SessionGreeting` (`03-widgets-and-types/02-component-library.md` §9), composed of exactly two things — a `text(role=title)` and a `field(input-mode=hybrid)` — matching the system's own boot pattern (`ui.greeting`, `ui.chat_input`, `ui.chat_send`) node-for-node:

```
stack#canvas.SessionGreeting dir=v align=center gap=lg
  text#ui.greeting(role=title size=title-2) "Welcome back"
  field#ui.chat_input(input-mode=hybrid placeholder="Ask or say what you need")
  button#ui.chat_send(icon=send label="Send")
```

- Renders at `elev=e0` (Principle 1 — this is a resting state, not a card floating over something).
- Entrance uses `motion.gentle` (`02-style/06-motion.md`) — this is an agent-initiated arrival, not something the person caused, so it follows `01-hig/04-inclusive-and-adaptive-design.md` §6's calmer-curve rule even on the very first patch of a session.
- The greeting's own text is the one place in this system's copy rules (`01-hig/03-content-and-voice.md` §1) where a little more warmth than a validation message would ever get is explicitly allowed — "Welcome back" is doing exactly the job Principle 4's real-time-locality and Principle 1's minimalism leave room for: one sentence, no filler before it, no filler after it.
- What happens next is the ordinary session loop — nothing about turn two onward is special-cased relative to turn one.

## 2. The Confirmation Surface

The Broker-owned template anatomy is fixed in `03-widgets-and-types/02-component-library.md` §8; this section is its visual design.

- **Elevation:** the reserved `elev=e4` tier (`02-style/05-materials-and-elevation.md` §1) — the only surface in the system permitted to use it, and the UI Runtime enforces that no agent-composed node can request it.
- **Token usage, deliberately shared, not distinct:** the Confirmation Surface uses the *same* `surface.overlay`, `text.*`, and `status.*` tokens as any other overlay in this system, rather than a bespoke "security chrome" look. A visually foreign, unfamiliar-looking prompt is easier to spoof convincingly with a look-alike than a prompt that's simply *unmistakably the same, consistent thing every time* — familiarity through consistency is the actual defense, not a deliberately alien appearance. What makes it unforgeable is that only the Confirmation Surface Daemon can render at `e4` and only the Broker's fixed templates author its copy (`01-hig/03-content-and-voice.md` §6) — never a visual signature an agent could try to imitate.
- **Motion is position-agnostic, because it has to be:** the real anti-automation mechanism randomizes the confirm action's position among three fixed locations and its label among a fixed set (`docs/components/policy-broker.md` § Randomization). Because the layout itself varies between instances, the surface's entrance/exit motion is a `scale`+fade using `motion.emphasized` in and `motion.exit` out, anchored to the surface's own center — never a directional slide tied to a specific corner, which would either be wrong for two of the three randomized positions or, worse, give away which position was chosen before the person has read anything.
- **The countdown is always visible**, using `type.family.numeric` (`02-style/03-typography.md` §6) so it doesn't visually jitter as it counts down, and its final several seconds SHOULD shift toward `status.warning` coloring so an inattentive person gets a redundant, non-color-alone cue (Principle 7) that time is running out before the automatic deny.
- **Nothing about it is dismissible the way a `dialog` is** — no outside-click-to-dismiss, no Escape shortcut that silently denies without the input-provenance check the Broker requires (`docs/components/compositor.md` § Provenance Marker). This is the one interactive surface in the entire system where "just tap outside to close it" is explicitly not a courtesy this design system extends.

## 3. `local-only mode`

The hard system setting (`docs/agent-core-spec.md` §9, `agent.local_only_mode`) that unconditionally closes the cloud-escalation path is represented by a persistent, unambiguous, always-visible system-level indicator — not folded into a settings menu where its state would be invisible most of the time. It uses `status.info` styling when off (cloud escalation permitted) and a distinct, deliberately calmer-than-`warning` treatment when on (an intentional, person-chosen restriction, not a problem) — paired with its own icon and the literal words "Local-only," never color alone, consistent with §5 below and with `05-ui-ux-patterns/02-feedback-and-status.md` §5.

## 4. Suggestion chips

`SuggestionTray` (`03-widgets-and-types/02-component-library.md` §9) is how the agent proposes something without being asked. Rules specific to this pattern, on top of the ordinary `SuggestionChip` definition:

- A suggestion MUST be dismissible individually (a small trailing dismiss affordance on the chip itself) without dismissing the whole tray — rejecting one proactive offer shouldn't cost the others.
- A suggestion's action, once pressed, behaves exactly like any other `on:press=mcp:` binding (`04-events/03-intent-routing.md`) — there is no different execution path for "something the agent offered" versus "something the person navigated to directly."
- Per `04-events/04-multimodal-and-voice-input.md` §4, every chip's action MUST be nameable and triggerable by voice with an identical result.
- A suggestion that would trigger a `CONFIRM`-gated action still goes through §2's Confirmation Surface exactly as if the person had found and pressed the equivalent control themselves — a proactive offer is never a shortcut around confirmation.

## 5. The processing-locus indicator

Resolving the promise made in `04-events/03-intent-routing.md` §4: a small, persistent, honest indicator — living in the same system-level status area as §3's local-only indicator, not per-message — shows whether the agent's *current* reasoning is happening on-device (`status.positive`-adjacent, calm) or has escalated to the cloud tier (`status.info`-adjacent, distinct hue from local-only's own indicator so the two are never confusable at a glance). This is independent of, and simpler than, whether any specific button press turned out to be lambda-fast or agent-reasoned (`04-events/03-intent-routing.md` §2–§3, which is deliberately *not* shown per-interaction) — it answers "where is my data going right now," which is a standing trust question, not "was that particular click cached."

## 6. Clarifying questions

When the agent needs more information before it can proceed, the question renders as an ordinary `text` node in the same conversational flow as the greeting, followed by whatever input affordance actually resolves it — a `field`, a `list(select=single)` of options, a `toggle` — never a generic yes/no `ConfirmDialog` used as a stand-in for a question that has more than two real answers. Per `01-hig/03-content-and-voice.md` §4, the question MUST NOT ask something the existing context already answers — an agent that asks a redundant question has failed at the one thing this whole system exists to save the person: their attention.

---

*Cross-references: `01-hig/01-design-principles.md` (Principle 1, 4, 7), `01-hig/03-content-and-voice.md` §1, §6 (greeting tone, unforgeable copy), `01-hig/04-inclusive-and-adaptive-design.md` §6 (agent-initiated motion), `02-style/05-materials-and-elevation.md` §1 (the `e4` tier), `03-widgets-and-types/02-component-library.md` §8–§9 (Confirmation Surface anatomy, `SessionGreeting`/`SuggestionTray`), `04-events/03-intent-routing.md` §4 (the processing-locus promise this resolves), `04-events/04-multimodal-and-voice-input.md` §4–§5, `docs/components/policy-broker.md` (the real mechanics this document gives a visual design to), `docs/agent-core-spec.md` §9 (`agent.local_only_mode`).*
