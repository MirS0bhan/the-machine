# Versioning and Lifecycle

This document set versions independently of both The Machine's overall release version and `ui-engine`'s own `0.1.0` module version (`ui-engine/models.py` et al.) — a design-system release can ship (new tokens, new components, clarified guidance) without an AUIL/ASL grammar change, and the grammar can change without every token needing to move.

## 1. Version scheme

`MAJOR.MINOR`, tracked in this document set's own `README.md` header:

- **MAJOR** — a change that could make existing, correct AUIL/ASL source render or behave differently (a token's hex value changing enough to matter, a mixin's default transition changing, a primitive's default role changing).
- **MINOR** — an addition (a new token, scale tier, mixin, component, pattern doc) that doesn't change how anything already written renders.

This document set is currently `0.1` — the same "design draft, not yet load-bearing on a shipped release" status the rest of the project's `-spec.md` family uses for pre-`1.0` work (`docs/agent-core-spec.md`, `lambda-server/docs/spec.md`, etc.).

## 2. Stability tiers

| Tier | Meaning | Where it's marked |
|---|---|---|
| **Stable** | The default for everything in this document set unless marked otherwise — safe for an agent to reach for in any production surface | (unmarked) |
| **Experimental** | Recently added, not yet validated across enough real screens to promise it won't change shape in a MAJOR bump | Marked inline at the point of definition — an agent SHOULD prefer a stable equivalent if one exists and MAY use an experimental one but should expect it to still move |

Nothing in this initial `0.1` release is marked experimental; the tier exists for future additions made through `07-governance/01-contribution-and-review.md`, not retroactively.

## 3. Deprecation

- A token, mixin, or component being replaced keeps its old name resolving to the *new* value/definition for at least one MINOR release after the replacement ships, with the old name's entry in the relevant file and in `06-glossary.md` marked "deprecated, resolves to `<new name>`" — an agent-authored patch using the old name still renders correctly during the overlap window; it just isn't the name new work should reach for.
- A deprecated name is removed outright only in a MAJOR release, and only after the overlap window has passed.
- Primitives are never deprecated by removal — per `01-hig/01-design-principles.md` Principle 1's "fixed set" framing, removing a primitive is a large enough change that it goes through the same bar as adding one (`07-governance/01-contribution-and-review.md` §1), not through this lighter deprecation path.

## 4. Changelog

Every MAJOR or MINOR release lists, at minimum: what was added, what was deprecated (with its replacement), and what — if anything — actually changed shape for already-written AUIL/ASL source. A release with nothing in the third category is the common case and is worth stating explicitly, since it's the difference between "safe to pick up immediately" and "read the migration notes first."

---

*Cross-references: `07-governance/01-contribution-and-review.md` (how a change gets approved before it's versioned here), `design-system/README.md` (this document set's current version header), `ui-engine/README.md` (the implementation's own, separately-tracked version).*
