# Primitive Types

AUIL's primitive tag set is fixed and small, on purpose — per `docs/spec.md` §3.6.2, "a small, fixed set of primitives... composable, not infinitely extensible per-request." Twelve primitives cover every screen this system renders; everything else in `03-widgets-and-types/02-component-library.md` is composition of these twelve plus ASL mixins, never a thirteenth tag.

A primitive's tag is always **lowercase** — this is how a reader (or the parser) tells a primitive apart from a mixin or a component name in a mixin-chain position, both of which are PascalCase (`design-system/README.md`, Notation).

## 0. Accessibility defaults at a glance

Every row below is enforced per `01-hig/02-accessibility.md` §1–§2: the Role column is fixed and non-overridable by an agent-authored patch; the Label column is the resolution order used when no explicit `label=` is given.

| # | Primitive | Role | Label resolves from |
|---|---|---|---|
| 1 | `stack` | `group` (or none, if purely structural) | Not directly labelable; `label=` promotes it to a labeled region |
| 2 | `grid` | `group` | Same as `stack` |
| 3 | `text` | `heading` (`role=title`) / `paragraph` (`role=body`) / `text` (`role=caption`) / `label` (`role=label`) | N/A — text is content, not a labelable control |
| 4 | `field` | `textbox` | **MUST** have a persistent, visible label — placeholder text never counts (`05-ui-ux-patterns/03-forms-and-data-entry.md` §1) |
| 5 | `button` | `button` | `label=`, else text child, else **mandatory** `label=` if icon-only |
| 6 | `list` | `list` (or `listbox`/`menu` when `select=` is set) | `label=` recommended when the list isn't already introduced by a preceding heading |
| 7 | `media` | content-dependent, always paired with a description | **MUST** have `label=` describing the content, independent of any caption text |
| 8 | `chart` | image-like, with a required text alternative | **MUST** have `label=` summarizing the data; decision-relevant data additionally needs a textual/tabular equivalent nearby, not just a label string |
| 9 | `icon` | `presentation` (decorative) unless standalone-interactive | Decorative by default; an icon used as the *entire* content of a `button` inherits that button's mandatory `label=` requirement |
| 10 | `slider` | `slider` (interactive) / `progressbar` (`interactive=false`) | `label=` required — a bare numeric range with no label is meaningless to assistive technology |
| 11 | `toggle` | `switch` / `checkbox` / `radio` (per `variant=`) | `label=`, else text child |
| 12 | `dialog` | `dialog` | `label=` (the dialog's own title) required — trapped focus with no announced title disorients exactly the audience focus-trapping is meant to help |

## 1. `stack`

The universal one-axis layout container. No visual properties beyond what a mixin adds — a bare `stack` is invisible structure.

- **Props:** `dir=` (`h` | `v`), `gap=` (a `space` scale reference), `pad=` (a `space` scale reference), `align=` (cross-axis: `start`|`center`|`end`|`stretch`), `distribute=` (main-axis: `start`|`center`|`end`|`between`).
- **Composition notes:** a hairline divider is a `stack.Divider` with no children — there is no separate divider primitive; the `Divider` mixin (`03-widgets-and-types/02-component-library.md` §0) supplies a fixed cross-axis size and a `border.default`-colored rule. `stack.Surface` is the base of nearly every visible container in `02-component-library.md`.
- **When not to use:** when content genuinely has two independent axes of relationship (rows *and* columns both matter) — that's `grid`.

## 2. `grid`

Two-axis layout container.

- **Props:** `columns=` (an integer, or omitted to resolve from the active size class, `02-style/07-layout-and-spacing.md` §2), `gap=`, and per-child `span=` (relative to the active column count, never a literal fraction).
- **When not to use:** for anything expressible as a single axis — reaching for `grid` where `stack` would do adds structure Principle 1 asks you to justify.

## 3. `text`

Renders text content. See `02-style/03-typography.md` for the full role/size reconciliation.

- **Props:** `role=` (`title`|`body`|`caption`|`label` — fixed, four values, accessibility-load-bearing), `size=` (the ten-step visual scale, e.g. `title-2`, `body`, `caption`), `align=`, `color=` (a `text.*` or `status.*` token — never a raw value).
- **Rule:** `role=title` MUST correspond to genuine document structure (a real heading in the content hierarchy). Wanting bigger text without heading semantics is `role=body size=title-3`, not `role=title` used for its size alone — conflating the two breaks screen-reader document outlines.

## 4. `field`

The single text/voice input primitive.

- **Props:** `value=` (typically `@`-bound, two-way), `placeholder=` (a hint, never a label substitute), `input-mode=` (`text` | `voice` | `hybrid` — `hybrid` shows a mic affordance alongside the text input, per the login-greeting pattern in `05-ui-ux-patterns/04-agent-presence-and-conversation.md`), `multiline=` (bool), `on:change=`.
- **States:** `idle`/`hover`/`focus`/`disabled`/`error` (`03-widgets-and-types/03-states-and-variants.md`).

## 5. `button`

A single discrete action.

- **Props:** `label=`, `icon=` (optional; MUST be paired with `label=` if it's the only content, per §0), `on:press=` (an `mcp:` sigil, almost always), `disabled=`, `variant=` (`primary`|`secondary`|`tertiary`|`destructive`|`ghost` — the emphasis/tone axis, `03-widgets-and-types/03-states-and-variants.md` §2).
- **Note on loading:** the press *feedback* (the `Pressable` mixin's squish) is always instant and local per Principle 4 — it never waits on the bound intent. If the bound action is slow, the button separately transitions into `state:loading` once the intent has fired, replacing its label with a determinate/indeterminate indicator; this is a data-bound transition, not a property of the press itself.

## 6. `list`

An ordered collection, optionally selectable or actionable. This is a load-bearing primitive: several composed controls that might look like they need their own primitive are actually `list` compositions.

- **Props:** `items=` (data-bound), `item-template=` (a component reference), `select=` (`none`|`single`|`multiple`), `on:select=`, `dir=` (`v` default, `h` for a chip rail or tab strip).
- **Composition notes:** a radio group is a `list(select=single)` of `toggle(variant=radio)` items; a checkbox group is a `list(select=multiple)` of `toggle(variant=checkbox)` items; a tab strip is a `list(dir=h select=single)` (`05-ui-ux-patterns/01-navigation-and-layout.md` §2). There is no separate radio-group, checkbox-group, or tab primitive because `list` already is one, correctly, for all three.

## 7. `media`

Renders a media stream. Never bundles transport controls itself — controls are sibling `button`/`slider` primitives composed alongside it (the real, existing pattern: `media#video` + `button#play` + `slider#progress` as siblings, wrapped by the `MediaPlayer` component, `03-widgets-and-types/02-component-library.md` §5).

- **Props:** `type=` (`video`|`audio`|`image`), `src=` (typically a `$lambda:` binding to a running lambda's stream output), `label=` (**mandatory** — an accessible description of the content, independent of any visible caption), `poster=` (for video, shown before playback starts).
- **States:** `loading`/`error`/`idle`, all `state:`-bound to the underlying stream's lifecycle (data-driven), never `on:`-bound (there's no raw input event for "the stream buffered").

## 8. `chart`

Data visualization.

- **Props:** `type=` (`line`|`bar`|`pie`), `data=` (typically `@`-bound to a State Store path), `axes=`.
- **Rule:** `label=` summarizing the data is mandatory (§0), and for anything decision-relevant (not purely decorative), the chart MUST be accompanied by an actual textual or tabular equivalent nearby — a well-written `label=` string is necessary but not sufficient for genuinely conveying trend or comparison data non-visually.

## 9. `icon`

See `02-style/04-iconography.md` for the full grid/size/color contract.

- **Props:** `name=` (an identifier from the icon set), `size=` (`icon.sm`|`icon.md`|`icon.lg`|`icon.xl`), `color=` (a token — never hardcoded), `variant=` (`outline`|`filled`).

## 10. `slider`

Continuous or discrete range display. Interactive by default; the primitive an agent reaches for for **both** an adjustable range control and a progress indicator, distinguished by one prop.

- **Props:** `min=`, `max=`, `value=` (`@`-bound, two-way when interactive), `step=`, `interactive=` (bool, default `true`), `indeterminate=` (bool — when `true`, ignores `value` and renders a looping fill instead, for unknown-duration progress).
- **Composition notes:** `slider(interactive=false value=@task.progress)` is a determinate progress bar. There is no separate progress-bar primitive because a read-only slider already is one.

## 11. `toggle`

Boolean or single-choice-within-a-group state.

- **Props:** `variant=` (`switch` default | `checkbox` | `radio`), `checked=` (`@`-bound, two-way), `on:change=`, `disabled=`.
- **Composition notes:** `variant=` changes the accessible role and the rendered shape (a pill switch vs. a square check vs. a round radio dot) but not the underlying boolean-state behavior — all three are the same primitive because, structurally, they are the same thing wearing three different accessible/visual conventions.

## 12. `dialog`

*Extends the currently-implemented primitive set (see `design-system/README.md`, "Relationship to the implemented AUIL/ASL grammar").* A modal surface with focus-trap and exclusive-input guarantees that don't reduce to a styling composition of `stack`.

- **Props:** `elev=` (fixed at `e3` — the UI Runtime enforces this, an agent cannot request a different tier for a `dialog`), `dismissible=` (bool — whether an outside click or Escape closes it, vs. requiring an explicit action), `on:dismiss=`.
- **Behavioral guarantees** (enforced by the UI Runtime, not re-implemented per component): traps Tab/Shift-Tab within its own interactive descendants; returns focus to the triggering control on close (`01-hig/02-accessibility.md` §7); renders a backdrop scrim at `opacity.overlay-scrim`; is the root of every member of the `Dialog` component family (`03-widgets-and-types/02-component-library.md` §6).
- **What a `dialog` is *not*:** the Confirmation Surface (`elev=e4`) is not an instance of this primitive and is never agent-instantiable — see `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §2. A `dialog` is something the agent can render; the Confirmation Surface is something only the Policy Broker can render, on a reserved layer this primitive does not reach.

---

*Cross-references: `01-hig/02-accessibility.md` §1–§2, §7 (the rules §0 and §4/§12 satisfy), `02-style/03-typography.md` (the `text` role/size model), `02-style/04-iconography.md` (the `icon` primitive's full contract), `03-widgets-and-types/02-component-library.md` (compositions built from these twelve), `03-widgets-and-types/03-states-and-variants.md` (the state machine every primitive above participates in), `04-events/01-event-model.md` (`on:`/`state:` transitions referenced throughout).*
