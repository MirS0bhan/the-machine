# Glossary

Every term this document set uses, defined once. This is a more detailed sibling to `docs/reference/glossary.md`, scoped specifically to the visual language; where the two overlap, this entry gives the fuller definition and the top-level glossary's short one still stands.

| Term | Definition | Defined in |
|---|---|---|
| **Agent Core** | The L4 harness — session loop, hybrid local/cloud model router, MCP client — that emits the AUIL/ASL patches this document set governs | `docs/agent-core-spec.md`, `docs/components/agent-core.md` |
| **AUIL** | Agent UI Layout — the line-oriented, indentation-based structure language (tags, ids, mixins, props, children) | `ui-engine/docs/spec.md` |
| **ASL** | Agent Style Language — the token/scale/motion/style-mixin language. Some earlier project documents expand this differently ("Agent State Language," "Adaptive Style Language"); this document set standardizes on Agent Style Language, matching the language's actual scope | `ui-engine/docs/spec.md`, `02-style/` |
| **Announce** (`announce=polite`/`assertive`) | The prop that marks a node's patches for live-region announcement to assistive technology; absent by default | `01-hig/02-accessibility.md` §8 |
| **Canvas** | The root surface rendered before any task-specific UI exists — where the greeting and persistent agent-presence affordances live | `05-ui-ux-patterns/01-navigation-and-layout.md` §4 |
| **Capability** (`CAP_*`) | A scoped permission a lambda or the Agent Core declares in its manifest and the Policy Broker grants, denies, or holds for confirmation | `docs/components/policy-broker.md` |
| **Component** | A named, PascalCase, registered AUIL tag built from a primitive root, mixins, and optional slots | `03-widgets-and-types/02-component-library.md` |
| **Compositor** | The L5 deterministic process that paints surfaces to pixels and owns the reserved Confirmation Surface role | `docs/components/compositor.md` |
| **Confirmation Surface** | The Broker-owned, `elev=e4`, agent-unreachable surface that renders capability/protected-unit confirmation prompts | `docs/components/policy-broker.md`, `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §2 |
| **Data-bound transition** | A `state:name → props` ASL transition driven by a variant flag or a State Store value, independent of any input event | `04-events/01-event-model.md` |
| **Density** | A surface-level setting (`comfortable`/`compact`) that modulates the effective `space` scale step without changing token names or `space.min-target` | `02-style/07-layout-and-spacing.md` §3 |
| **Dialog** (primitive) | The twelfth, focus-trapping, exclusive-input modal primitive this document set adds to the implemented set | `03-widgets-and-types/01-primitive-types.md` §12 |
| **Elevation** (`elev`) | The five-tier (`e0`–`e4`) ordinal scale signaling stacking hierarchy, tied to shadow (light theme) or lightness (dark theme) and to the compositor's real `z_order` | `02-style/01-design-tokens.md` §3, `02-style/05-materials-and-elevation.md` |
| **Fallback Shell** | The zero-inference recovery UI, outside this token system entirely, that renders when the Agent Core itself is unavailable | `docs/components/fallback-shell.md`, `02-style/05-materials-and-elevation.md` §5 |
| **Input-triggered transition** | An `on:event => props` ASL transition driven by a raw pointer/keyboard event, resolved entirely locally | `04-events/01-event-model.md` |
| **Intent event** | An event that crosses the MCP bus via a bound `mcp:`/`$lambda:` sigil, as opposed to a motion event | `01-hig/01-design-principles.md` Principle 4, `04-events/01-event-model.md` |
| **Lambda** | A sandboxed, registered function on the Lambda Execution Server that an intent may resolve to | `docs/components/lambda-server.md` |
| **Local-only mode** | The hard system setting that unconditionally closes cloud escalation, independent of the agent's own routing judgment | `docs/agent-core-spec.md` §9, `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §3 |
| **Mixin** | A named, PascalCase ASL style definition (base properties plus state transitions) applied to a primitive or component via dot-chain syntax | `03-widgets-and-types/02-component-library.md` §0 |
| **Motion curve** | A named `spring`/`duration` recipe (`snappy`, `gentle`, `standard`, `emphasized`, `exit`, `reduced`) referenced by bare name from an `on:`/`state:` transition | `02-style/06-motion.md` |
| **Motion event** | An `on:event =>` transition that never crosses the MCP bus | `01-hig/01-design-principles.md` Principle 4 |
| **MCP** | Model Context Protocol — the one IPC/audit fabric every component, including the Agent Core, uses | `docs/architecture/philosophy.md` commitment 7 |
| **Opacity token** | A category of alpha-multiplier tokens (`opacity.border`, `.dim`, `.disabled`, `.overlay-scrim`), each with a mandatory high-contrast variant | `02-style/01-design-tokens.md` §2 |
| **Patch protocol** | The five operators (`~ + - ! @`) that update the live UI State Tree without a full re-render | `ui-engine/docs/spec.md` |
| **Policy Broker** | The L2 deterministic, non-LLM gatekeeper that mediates every capability request, including everything this document set's Confirmation Surface renders | `docs/components/policy-broker.md` |
| **Primitive** | One of the twelve fixed, lowercase AUIL tags this whole component system composes from | `03-widgets-and-types/01-primitive-types.md` |
| **Privacy tag** | A structural flag on a wake context that excludes the cloud model tier from a given turn, regardless of the local model's confidence | `docs/agent-core-spec.md` §4 |
| **Processing locus (indicator)** | The persistent UI signal showing whether the agent's current reasoning is local or cloud | `05-ui-ux-patterns/04-agent-presence-and-conversation.md` §5 |
| **Protected unit** | A load-bearing systemd/service unit that cannot be stopped/restarted/disabled without Confirmation Surface approval | `docs/components/policy-broker.md` § Protected Units |
| **Radius scale** (`radius`) | The `xs`–`xl`, `full` corner-rounding scale, referenced inline with the `r-` shorthand | `02-style/01-design-tokens.md` §3 |
| **Reference sigil** | One of `$lambda:`, `mcp:`, `@` — a property-value prefix that points outside the literal | `design-system/README.md`, Notation; `04-events/03-intent-routing.md` |
| **Role** (accessible) | The fixed, non-overridable-by-default accessibility role every primitive declares | `01-hig/02-accessibility.md` §2, `03-widgets-and-types/01-primitive-types.md` §0 |
| **Scale** | A named ramp of tiers (`radius`, `space`, `elev`) declared with `scale name: tier=value ...` | `02-style/01-design-tokens.md` §3 |
| **Sheet** | An edge-anchored auxiliary panel, modal or non-modal | `03-widgets-and-types/02-component-library.md` §1, `05-ui-ux-patterns/06-multitasking-and-surfaces.md` §3 |
| **Size class** | `compact`/`standard`/`expansive` — resolved from a surface's available inline-axis space, not a device category | `02-style/07-layout-and-spacing.md` §1 |
| **Slot** | A named, optionally-required insertion point in a component definition | `03-widgets-and-types/02-component-library.md` |
| **Space scale** (`space`) | The `xxs`–`huge` spacing scale, referenced inline with the `s-` shorthand | `02-style/01-design-tokens.md` §3 |
| **State Store** | The L1 persistent, patch-addressed store holding the UI State Tree and system/task/preference/permission state | `docs/components/state-store.md` |
| **Status token** | One of `status.positive`/`.warning`/`.destructive`/`.info`, each with solid/subtle/on-solid/on-subtle variants and a mandatory paired icon | `02-style/02-color-and-surfaces.md` §4 |
| **Surface** | The canonical term for a compositor-addressed UI container — deliberately used in place of "window" | `01-hig/03-content-and-voice.md` §3, `docs/components/compositor.md` |
| **Tier A / Tier B** | The local (always-resident) and cloud (escalation-only) model tiers the Agent Core routes between | `docs/agent-core-spec.md` |
| **Token** | A single named `category.role` value (color, fixed spacing, family, etc.), declared with `token category.role = value` and referenced with `token:` | `02-style/01-design-tokens.md` §1 |
| **UI Runtime** | The L5 deterministic renderer that consumes the UI State Tree and applies patches | `docs/components/ui-runtime.md` |
| **UI State Tree** | The declarative document, addressed by stable node ids, that the UI Runtime renders and the Agent Core patches | `docs/architecture/layers.md` §1.2 |
| **Variant** | An authored (not runtime-triggered) styling axis — size, emphasis/tone, density — orthogonal to interaction state | `03-widgets-and-types/03-states-and-variants.md` §5 |
| **Vibrancy** | A backdrop-blur material (`thin`/`regular`/`thick`) layered onto a base surface token via the `+ vibrancy()` token derivation | `02-style/01-design-tokens.md` §4, `02-style/05-materials-and-elevation.md` §2 |

---

*Cross-references: every file in `01-hig/` through `05-ui-ux-patterns/` — this glossary intentionally duplicates no definition's substance, only its pointer.*
