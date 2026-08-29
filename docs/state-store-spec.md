# State Store — UI State Tree, System/Task State, Persistence & Subscriptions

**Fills:** §3.2.2 of `agent-native-os-architecture.md` (State Store)
**Related:** `auil-asl-spec.md` §2.2 (`@` sigil / path binding), §4 (patch protocol), §3.5 (`state:*` ASL bindings); `lambda-server-spec.md` §2.1 (`CAP_STATE_READ`/`CAP_STATE_WRITE`), §4.3 (SDK `state.get`/`state.set`); `policy-broker-spec.md` §11 (path-scoped grant validation)
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation

---

## 0. Design goals

1. **Incremental, not throwaway.** The entire premise of the State Store per the parent doc is that agent output survives between invocations — scroll position, half-typed text, playback position. Every design choice here optimizes for cheap partial reads/writes over cheap full snapshots.
2. **One addressing scheme, used everywhere.** The `@path` sigil in AUIL (`auil-asl-spec.md` §2.2) and the `state.get`/`state.set` calls in the Lambda SDK (`lambda-server-spec.md` §4.3) must resolve against the *same* path grammar — there is exactly one State Store, not a UI-flavored one and a task-flavored one that happen to look similar.
3. **Capability-gated, not trust-gated.** Every read or write is checked against `CAP_STATE_READ(paths=[...])` / `CAP_STATE_WRITE(paths=[...])` from the caller's manifest, enforced the same way the Lambda Server enforces `CAP_IPC_CALL` — a scoped grant, checked before the call is served, not after.
4. **Reactive by default.** Because the Event/Scheduler Bus (`event-bus-spec.md`) and ASL's `state:*` bindings (`auil-asl-spec.md` §3.5) both need to react to state changes without agent involvement, the Store is a pub/sub system with a KV interface, not a KV system with pub/sub bolted on.
5. **Survives a crash without an agent.** The Fallback Shell (parent §3.7) reads "the last known-good State Tree" with zero inference running — persistence has to be durable and independently readable, not something only the Agent Core knows how to reconstruct.

---

## 1. Data model

Single hierarchical store, dot-separated path addressing, four top-level namespaces:

```
ui.<tree>          — the UI State Tree(s) the Runtime renders (one per active surface/window)
task.<...>         — running task list, active session metadata, intent history
prefs.<...>        — user preferences (volume, theme overrides beyond token defaults, local-only flag)
perm.<...>         — permission-grant records (mirrors Broker grant tokens for UI-visible "what has access to what")
```

- Leaf values are typed scalars, strings, lists, or nested objects — the store does not distinguish "UI value" from "task value" structurally; the namespace prefix is the only thing that separates them, which is what lets `@player.position` (task-ish) and `@prefs.volume` (prefs) both resolve through one sigil in AUIL without the parser caring which namespace it's in.
- The `ui.<tree>` namespace additionally stores the tree in a **node-addressable** form (each AUIL node's runtime-assigned or explicit id is a first-class path segment: `ui.root.controls.play`), because patch ops (§8) address nodes by id, not by arbitrary path — this is the one namespace with a fixed internal shape (mirroring the AUIL tree), while `task`/`prefs`/`perm` are free-form nested documents.

---

## 2. Path addressing & the `@` contract

- A path is a dot-separated sequence of segment names; array indices use `[n]`.
- `@path` in AUIL is exactly a State Store path with the `ui.`/`task.`/`prefs.`/`perm.` prefix *implied by context* where unambiguous (e.g. `@player.position` inside a UI patch resolves to `task.player.position` if `player` isn't a node id in the current tree — resolution order is: current-tree node id first, then `task`, then `prefs`, deterministic and documented, never guessed by an LLM at authoring time since the agent doesn't need to know which namespace it landed in to read/write it).
- Two-way binding (`field value=@prefs.volume`) means the UI Runtime both reads this path to render and writes to it on user input directly — this write bypasses the Agent Core entirely (it's a motion-adjacent, not intent-adjacent, event per `auil-asl-spec.md` §3.4) but does **not** bypass the Store's own capability check: the UI Runtime itself holds a broad, Broker-issued `CAP_STATE_WRITE` grant scoped to non-sensitive UI-reflection paths, established once at Runtime startup, not re-checked per keystroke.

---

## 3. Patch & versioning model

- Every write is internally a patch, never a blind overwrite — even a `state.set` call is recorded as "path X: old value → new value" so the Store can answer "what changed since revision N," which the Event Bus needs (§ below) to avoid re-scanning full documents for diffs.
- **Write-ahead log (WAL) + periodic snapshot**, same durability shape as any embedded transactional store: every accepted write is appended to the WAL and fsync'd before being acknowledged to the caller; a background compactor periodically folds the WAL into a snapshot so recovery doesn't replay from empty.
- **Revision numbers are global and monotonic**, not per-path — this is what lets `state.watch` (§4) hand out "give me everything since revision N" cheaply, and it's what lets the Fallback Shell (parent §3.7) say "render whatever the store held at the last fully-committed revision" with no ambiguity about partial writes.
- Rollback: `state.history(path, limit)` and `state.rollback(path, revision)` exist for `task`/`prefs`/`perm` namespaces (undo-style recovery); the `ui.<tree>` namespace deliberately does **not** expose rollback as a general operation — a full-tree revert of live UI is exactly the `!` replace case the AUIL patch protocol already treats as a last resort (`auil-asl-spec.md` §4), not a State Store primitive.

---

## 4. Subscriptions (`state.watch`)

- `state.watch(path_prefix, since_revision?)` opens a subscription that yields patch events (path, old, new, revision) as they occur under that prefix.
- This is the mechanism behind ASL's `state:*` bindings (`auil-asl-spec.md` §3.5): a `Loading` style's `state:loading` transition watches a lambda's declared health path (e.g. `task.functions.video_player.health`) and the UI Runtime re-renders the bound node purely from the watch callback — no MCP round-trip to the Agent Core, no inference.
- The Event/Scheduler Bus (`event-bus-spec.md`) is itself a long-lived `state.watch` client on `task.functions.*.health` and select `perm.*` paths, which is how a lambda crash becomes an event-bus notification without the Store needing to know anything about "events" as a concept — it only knows about patches and subscribers.
- Subscriptions are capability-gated identically to reads: watching a path requires the same `CAP_STATE_READ` grant that a one-shot `state.get` on that path would require.

---

## 5. Capability gating

- Every `state.get`/`state.set`/`state.watch` call carries the caller's identity (lambda name, or "agent-core", or "ui-runtime" as fixed system identities); the Store checks the caller's granted paths (from the Broker, `policy-broker-spec.md` §4) before serving.
- Grants are **prefix-scoped**: `CAP_STATE_WRITE(paths=["task.player.*"])` permits writes under that prefix and nothing outside it, checked as a literal prefix match, not a semantic one — this mirrors the Lambda Server's `CAP_IPC_CALL` philosophy of declared-edge, not blanket-capability.
- The `perm.*` namespace is additionally locked to Broker-only writes at the Store level (not just policy-level) — no manifest, however broad, can request `CAP_STATE_WRITE(paths=["perm.*"])` and have it granted, because the Store itself refuses that grant shape regardless of what the Broker says, as a defense-in-depth backstop mirroring the Broker's own protected-unit hard-wire (`policy-broker-spec.md` §5).

---

## 6. MCP surface

| Tool | Purpose |
|---|---|
| `state.get(path)` | Point read. |
| `state.set(path, value)` | Point write (internally a patch, §3). |
| `state.patch(ops)` | Batch of `{path, value}` ops applied atomically — used by the UI Runtime to apply an AUIL patch op-list (§8) as one revision instead of N. |
| `state.watch(path_prefix, since_revision?)` | Subscribe to changes (§4); long-lived MCP stream. |
| `state.history(path, limit)` | Recent revisions for a path (`task`/`prefs`/`perm` only). |
| `state.rollback(path, revision)` | Revert a path to a prior revision (`task`/`prefs`/`perm` only). |
| `state.snapshot()` | Force a snapshot write (used by the Fallback Shell / recovery tooling, not normal traffic). |

---

## 7. Consistency & concurrency

- **Single-writer-per-path, last-write-wins across writers**, with the global revision counter making "last" unambiguous — this is deliberately weaker than full transactional isolation because the workload (UI reflection, task bookkeeping) doesn't need serializability, and adding it would cost latency on the hottest path in the system (every UI patch touches this store).
- `state.patch`'s atomicity guarantee is scoped to "all-or-nothing within one revision," not "isolated from concurrent readers mid-application" — readers either see the pre-patch or post-patch state, never a partial patch, but there's no broader transaction concept beyond that.
- Conflicting concurrent writes to the *same* path from two callers are resolved by arrival order at the Store, not by caller priority — a caller that needs stronger guarantees (e.g. "only I may currently be writing `task.player.position`") gets that from its `CAP_STATE_WRITE` grant being the *only* grant issued for that prefix, which is a Broker-level guarantee, not a Store-level one.

---

## 8. Integration with the AUIL patch protocol

AUIL's five patch ops (`auil-asl-spec.md` §4) map onto Store operations against the `ui.<tree>` namespace as follows:

| AUIL op | Store operation |
|---|---|
| `~id(props)` | `state.patch` with one `{path: ui.tree.id.props.*, value}` entry per changed prop |
| `+anchor: node` | `state.patch` inserting a new subtree under `ui.tree.anchor.children[n]` |
| `-id` | `state.patch` removing `ui.tree.id` and all descendants in one op |
| `!id: node` | `state.patch` replacing the subtree wholesale — the Store does not attempt to diff this into smaller ops itself (that question is explicitly left open in `auil-asl-spec.md` §9.6; this spec takes no position beyond "the Store executes what it's given") |
| `@id → other-id` | `state.patch` as a move (same subtree, new parent path) — the Store's revision log records this as a move, not a delete+insert, so `state.watch` subscribers on the moved subtree don't see a spurious delete event |

The UI Runtime is the only component that ever writes to `ui.<tree>`; the Agent Core never calls `state.set` on UI paths directly — it emits AUIL patch text over MCP (`ui.patch`), which the Runtime parses and translates into `state.patch` calls. This keeps the Store's `ui.<tree>` schema an implementation detail the Agent Core doesn't need to know precisely, matching the "agent decides what, not how" commitment (parent §1).

---

## 9. Security summary

| Threat | Mitigation |
|---|---|
| Lambda reads/writes state outside its declared scope | Prefix-scoped `CAP_STATE_READ`/`WRITE` grants, checked per call (§5) |
| Privilege-escalation via `perm.*` writes | Store-level hard refusal of any grant shape touching `perm.*` except the Broker's own identity, independent of policy (§5) |
| Crash mid-write corrupts state | WAL + fsync before ack; snapshot/replay recovery (§3) |
| Fallback Shell reads inconsistent state after a crash | Global monotonic revision counter means "last committed revision" is always well-defined, even without the Agent Core running (§3) |
| Watch subscriber flooding | `state.watch` calls are themselves capability-gated and subject to the Broker's per-identity rate limiting (`policy-broker-spec.md` §6) |

---

## 10. Open items before implementation

1. **On-disk format** — embedded engine choice (e.g. an LSM-tree store vs. a simpler append-log + in-memory index) given the write-heavy, small-value workload pattern.
2. **Snapshot compaction cadence** — time-based vs. WAL-size-based trigger, and whether it should ever run while `state.watch` subscribers are active without pausing delivery.
3. **Multi-tree `ui.<tree>` lifecycle** — when a UI surface (window) closes, does its tree get pruned immediately, retained for N days, or retained until explicit `state` cleanup — ties into eventual multi-user/multi-surface work flagged as out of scope in parent §7.7.
4. **`state.patch` op size limits** — a pathological `!` replace on a huge subtree needs a ceiling so one bad patch can't stall the Store for other callers.
5. **Path grammar formalization** — this doc describes the `@` resolution order informally (§2); needs an actual grammar shared with the AUIL parser so both sides agree byte-for-byte on ambiguous cases.
