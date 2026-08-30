# Component Library

Components are named, registered AUIL tags (PascalCase — `design-system/README.md`, Notation) built from a `parent` (for inheritance), a set of ASL mixins, and optional slots — exactly the registration shape already implemented in `ui-engine/components.py`. Eight components are registered there today (`Surface`, `Card`, `ListRow`, `PrimaryButton`, `IconBtn`, `Field`, `MediaPlayer`, `Chart`); this document keeps all eight exactly as defined and completes the rest of the catalog a full desktop needs.

## 0. Mixins referenced throughout this catalog

These are the literal recipes `03-widgets-and-types/03-states-and-variants.md` §4 maps to states. Defined once, here, rather than repeated per component.

| Mixin | Trigger | Applies | Motion |
|---|---|---|---|
| `Surface` | (base — always present, no trigger) | `bg=token:surface.card`, `radius=r-lg`, `elev=e1` | — |
| `Hoverable` | `on:hover` | Elevation steps up one tier; `scale=1.02` | `snappy` |
| `Pressable` | `on:press` | `scale=0.97` | `snappy` |
| `Focusable` | `on:focus` | 2px `border.focus` ring, offset from the edge | `snappy` |
| `Disableable` | `state:disabled` | Whole node at `opacity.disabled` | `reduced` |
| `Selectable` | `state:selected` | `bg=token:accent.subtle` (or a leading `border=token:accent.default` bar for row-style items — component-specific) | `standard` |
| `Loading` | `state:loading` | Primary content replaced or overlaid by an indeterminate `slider`/`Spinner` | `gentle` |
| `Divider` | (base, no trigger) | `bg=token:border.default`; fixed 1px cross-axis size, `stack` sizing keyword `fixed` | — |
| `Spinning` | `state:loading` | Continuous rotation of an `icon` | `gentle`, looping |

## Slot defaults (resolves `ui-engine/README.md` open item 2)

Every slotted component's `SlotDefinition` MAY carry `default_children`. When a slot is left unfilled at the point of use:

1. If the component definition supplies `default_children` for that slot, the UI Runtime renders those.
2. Otherwise, if the slot is `required` (the default), the omission is a validation failure at the same tier as a malformed patch op or an empty accessible label (`01-hig/02-accessibility.md` §1) — the UI Runtime MUST refuse to render the instance, not silently render a gap.
3. If the slot is explicitly `required=false`, it simply renders nothing.

## Adding a mixin at runtime (resolves `ui-engine/README.md` open item 4)

A `~id(mixins="Surface.Selected")` update patch replaces a node's mixin list using the ordinary `~` update operator — mixins are just another node attribute, so no separate "style attachment" patch operation exists or is needed. Setting `mixins=` this way follows the same last-applied-wins conflict rule as authoring it inline.

## 1. Containers

| Component | Root | Mixins | Slots | Role | Purpose |
|---|---|---|---|---|---|
| `Surface` *(real)* | `stack` | `Surface` | — | `group` | The base styled container everything else in this section extends |
| `Card` *(real)* | `stack` | `Surface`, `Hoverable` | — | `group` | Default container for one distinct, unrelated chunk of content — per Principle 1, add only when a boundary is genuinely needed |
| `Sheet` | `stack` | `Surface` | `header`, `body` (required) | `dialog` when modal, `region` when non-modal | A large auxiliary panel anchored to a surface edge (settings, details-of-an-item) — see `05-ui-ux-patterns/06-multitasking-and-surfaces.md` §4 for its entrance/exit choreography and modal-vs-non-modal decision |

## 2. Navigation

| Component | Root | Mixins | Slots | Role | Purpose |
|---|---|---|---|---|---|
| `NavList` | `list(dir=v select=single)` | `Selectable` per item | — | `list`/`listbox` per items | Primary in-surface navigation; collapses to icon-only at `compact` size class (`03-widgets-and-types/04-composition-and-responsive-layout.md`) |
| `TabBar` | `list(dir=h select=single)` | `Selectable` per item | — | **`tablist`**, items `tab` — an explicit role override (`01-hig/02-accessibility.md` §2) since `list`'s default role would otherwise apply | Switches a surface's primary content between a small, fixed set of peer views |
| `SegmentedControl` | `list(dir=h select=single)` | `Selectable` per item | — | `radiogroup`, items `radio` | Switches a *local* view or filter within a region — narrower scope than `TabBar`, which switches the whole surface's primary content |
| `Breadcrumb` | `list(dir=h)` | — | — | `navigation` | Hierarchical path; items separated by a small directional `icon` (mirrors per `02-style/04-iconography.md` §4) |
| `AppBar` | `stack(dir=h)` | `Surface` | `leading`, `title` (required), `actions` | `group` | A surface's header chrome; `actions` overflow into a `Menu` at `compact` size class rather than clipping (`03-widgets-and-types/04-composition-and-responsive-layout.md`) |

## 3. Actions

| Component | Root | Mixins | Slots | Role | Purpose |
|---|---|---|---|---|---|
| `PrimaryButton` *(real)* | `stack` | `Surface`, `Hoverable`, `Pressable` | — | `button` | Default `variant=primary` action button |
| `IconBtn` *(real)* | `stack` | `Surface`, `Hoverable`, `Pressable` | — | `button` | Icon-only action; `label=` mandatory (`01-hig/02-accessibility.md` §1) |
| `ListRow` *(real)* | `stack` | `Surface`, `Hoverable`, `Pressable` | — | `button` or `listitem`, context-dependent | A tappable row — the base of `CheckboxRow`/`RadioRow` (§4) and most `list` item templates |
| `SuggestionChip` | `stack(dir=h)` | `Surface`, `Hoverable`, `Pressable` | — | `button` | An agent-proposed quick action (`05-ui-ux-patterns/04-agent-presence-and-conversation.md` §4); `radius=r-full` |
| `Tag` | `stack(dir=h)` | `Surface` | — | `group` (or `status` when bound to a live value) | A small categorical or status label; not interactive by default — an interactive/dismissible variant adds `Hoverable`+`Pressable` explicitly at the point of use, per `03-widgets-and-types/03-states-and-variants.md` §4's "compose explicitly" pattern |

## 4. Forms

| Component | Root | Mixins | Slots | Role | Purpose |
|---|---|---|---|---|---|
| `Field` *(real)* | `field` | `Surface` | `label` (required), `input` (required) | `textbox` | Base labeled input — keeps its exact existing slot definition |
| `SearchField` | `field` (parent `Field`) | `Surface` | `label`, `input` | `searchbox` | `Field` plus a leading search `icon` and a trailing clear action that appears only once `value` is non-empty |
| `CheckboxRow` / `RadioRow` | `stack` (parent `ListRow`) | `Surface`, `Hoverable`, `Pressable` | `label` (required) | `checkbox` / `radio` (matches the wrapped `toggle`'s `variant=`) | The standard tappable row pairing a `toggle` with its label — the whole row, not just the control, is the hit target |
| `FieldGroup` | `stack(dir=v)` | — | `label`, `field` (required), `help` | `group` | Binds a label, a `field` (or `toggle`/`slider`), and optional help/error text into one reviewable unit — see `05-ui-ux-patterns/03-forms-and-data-entry.md` |

## 5. Data and media

| Component | Root | Mixins | Slots | Role | Purpose |
|---|---|---|---|---|---|
| `MediaPlayer` *(real)* | `stack` | `Surface` | `video` (required), `controls` (required) | `group` (contains a `media` + transport `button`/`slider` siblings) | Keeps its exact existing slot definition |
| `Chart` *(real)* | `stack` | `Surface` | `data` (required) | `group` (wraps the `chart` primitive and its required text alternative) | Keeps its exact existing slot definition |
| `ProgressCard` | `stack` (parent `Card`) | `Surface`, `Loading` | `title` (required), `detail` | `status` | Live status of one background task (a lambda invocation, a download) — pairs a `slider(interactive=false)` with descriptive text |

## 6. Feedback and overlays

| Component | Root | Mixins | Slots | Role | Announce default (`01-hig/02-accessibility.md` §8) |
|---|---|---|---|---|---|
| `Toast` | `stack(dir=h)` | `Surface` | `icon`, `text` (required), `action` | `status` | `polite`; auto-dismisses, never `assertive` (`05-ui-ux-patterns/02-feedback-and-status.md` §1) |
| `Banner` | `stack(dir=h)` | `Surface` | `icon`, `text` (required), `actions` | `status` (or `alert` for `severity=destructive`) | `polite` for informational, `assertive` for errors requiring action |
| `Tooltip` | `stack` | `Surface` (with `+ vibrancy(thin)`) | `text` (required) | `tooltip` | never — tooltips are supplementary, not new information |
| `Menu` | `list(select=single)` | `Surface` (`+ vibrancy(regular)`) | — | `menu`, items `menuitem` | never |
| `Spinner` | `icon` | `Spinning` | — | `status` | `polite`, once, on appearance/disappearance — not continuously while spinning |
| `EmptyState` | `stack(dir=v align=center)` | — | `icon`, `title` (required), `body`, `action` | `group` | Full treatment in `05-ui-ux-patterns/05-empty-loading-and-error-states.md` |
| `Skeleton` | `stack` (parent `Surface`) | `Surface`, `Loading` | — | `presentation` | never — a skeleton is a loading placeholder, not new information |

## 7. Dialog family

All four are rooted at the `dialog` primitive (`03-widgets-and-types/01-primitive-types.md` §12) and therefore inherit its focus-trap, focus-return, and backdrop guarantees automatically.

| Component | Slots | Actions | Use |
|---|---|---|---|
| `AlertDialog` | `title` (required), `body` (required) | Single acknowledge action | An informational message that must be seen and dismissed — no decision to make |
| `ConfirmDialog` | `title` (required), `body` (required) | Confirm + cancel | An agent-initiated, non-protected decision ("Discard unsaved changes?") — **not** a substitute for the Broker-owned Confirmation Surface (§8) for anything capability- or protected-unit-related |
| `FormDialog` | `title` (required), `body` (a form, required), `actions` | Submit + cancel | A short, focused data-entry task that doesn't need a full surface |
| `Sheet` (non-modal) | see §1 | — | Prefer over a `dialog`-family component when the content doesn't need to interrupt — a `Sheet` at `elev=e2` beside content, rather than a `dialog` at `elev=e3` blocking it, per Principle 1 |

## 8. The Confirmation Surface's anatomy (documented, not agent-instantiable)

This is the one entry in this catalog with no AUIL tag and no `parent`/`mixins`/`slots` registration — it is rendered exclusively by the Confirmation Surface Daemon (`docs/components/policy-broker.md` § Confirmation Surface; `docs/architecture/philosophy.md` commitment 10), never by an agent-authored patch, at the reserved `elev=e4` tier (`02-style/05-materials-and-elevation.md` §1). It is documented here, once, so its visual anatomy is defined in the same place as everything else it visually relates to, and so `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §2 has a single anatomy to point at.

Its structure is fixed by the Broker's real template schema and MUST render every field:

| Field | Content |
|---|---|
| Header | A fixed warning glyph + "Capability Request" (or the template's own fixed title) |
| Requester | The real identity of the component asking (never agent-composed prose) |
| Description | The real, specific description of the request |
| Capability / Scope | The exact capability and scope being requested |
| Confirm action | Label rotates among a fixed set (`01-hig/03-content-and-voice.md` §3) for anti-automation; position rotates among three fixed positions for the same reason |
| Deny action | Always present, always labeled `"Deny"` — the one string in this template that does not rotate |
| Timer | A visible countdown to the fixed timeout, after which the request resolves to deny |

Every value in every field is read directly from the request the Broker is mediating — never summarized, paraphrased, or composed by a model. This is what `01-hig/03-content-and-voice.md` §6 means by "unforgeable."

## 9. Agent-presence components

| Component | Root | Mixins | Slots | Purpose |
|---|---|---|---|---|
| `SessionGreeting` | `stack(dir=v)` | `Surface` | `text` (required), `input` (required) | The first patch the person sees post-login (`05-ui-ux-patterns/04-agent-presence-and-conversation.md` §1) — composes a `text(role=title)` greeting with a `field(input-mode=hybrid)`, matching the real boot pattern's `ui.greeting`/`ui.chat_input`/`ui.chat_send` node IDs |
| `SuggestionTray` | `list(dir=h)` | `Surface` (`+ vibrancy(regular)`) | — | A horizontal rail of `SuggestionChip` items the agent offers proactively |

---

*Cross-references: `01-hig/02-accessibility.md` (role overrides, slot/label validation-failure tier), `01-hig/03-content-and-voice.md` §6 (why §8's copy is unforgeable), `03-widgets-and-types/01-primitive-types.md` (the twelve primitives every component here is rooted in), `03-widgets-and-types/03-states-and-variants.md` (the state model §0's mixins implement), `02-style/05-materials-and-elevation.md` (`elev=e4` reservation), `05-ui-ux-patterns/02-feedback-and-status.md` and `05-ui-ux-patterns/04-agent-presence-and-conversation.md` (usage patterns for §6–§9).*
