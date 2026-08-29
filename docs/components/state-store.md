# State Store

**Layer:** L1  
**Type:** Deterministic, non-LLM  
**Language:** Rust  
**Dependencies:** RocksDB (or sled) for storage  

---

## Overview

The State Store is a **persistent, structured store** for two kinds of state:

1. **UI State Tree** — the declarative document the UI Runtime renders
2. **System/Task State** — running task list, function registry, permission grants, conversation/intent history, user preferences

The agent *patches* the UI tree; it does not regenerate it from scratch each turn. Every write is internally a patch (old → new), with a global, monotonic revision number.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  State Store                                                        │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  In-Memory State                                               │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ UI Trees    │  │ Task State  │  │ System State        │ │ │
│  │  │ (ui.*)      │  │ (task.*)    │  │ (prefs.*, perm.*)    │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Storage Engine (RocksDB)                                    │ │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────┐ │ │
│  │  │ WAL     │  │ MemTable │  │ SST     │  │ Manifest        │ │ │
│  │  │         │  │         │  │ Files   │  │                 │ │ │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Subscriptions                                               │ │
│  │  ┌─────────────────────────────────────────────────────────┐ │ │
│  │  │  path_prefix → [Subscriber1, Subscriber2, ...]            │ │ │
│  │  └─────────────────────────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  MCP Interface: state.get, state.set, state.patch, state.watch│ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Data Model

### Namespaces

The State Store uses a **single hierarchical store** with dot-separated path addressing and four top-level namespaces:

| Namespace | Prefix | Description | Write Access |
|-----------|--------|-------------|--------------|
| UI State | `ui.<tree>` | Declarative UI trees | Agent Core, UI Runtime |
| Task State | `task.*` | Tasks, sessions, intent history | Agent Core, Lambda Server |
| Preferences | `prefs.*` | User preferences, themes | Agent Core, User |
| Permissions | `perm.*` | Permission grant records | Policy Broker only |

### Key Structure

All keys are UTF-8 strings with the format `{namespace}.{path}`.

**Examples:**
- `ui.root.controls.play.text` — text content of a play button
- `ui.root.controls.play.style` — style of the play button
- `task.functions.video_player.health` — health status of video player lambda
- `task.intents.0` — first intent in history
- `prefs.theme.colors.primary` — primary color preference
- `prefs.asl.mixins.Card` — ASL mixin definition
- `perm.grants.lambda-video-player` — capability grants for video player

### Special Keys

| Key | Type | Description |
|-----|------|-------------|
| `__revision` | u64 | Global monotonic revision counter |
| `__last_snapshot` | u64 | Revision of last snapshot |
| `__boot_time` | i64 | Unix timestamp of store initialization |

---

## Storage Engine

### Choice: RocksDB

**Rationale:**
- Write-heavy workload (UI patches are frequent)
- Need range scans for `state.watch`
- Snapshot/compaction built-in
- Mature, production-tested

**Alternatives considered:**
- **sled** — pure Rust, but less mature
- **SQLite** — not optimized for this access pattern
- **Custom LSM** — too much work, RocksDB already solves this

### On-Disk Layout

```
/var/lib/the-machine/store/
├── CURRENT              # Current manifest version
├── MANIFEST-xxx         # SST file manifest
├── *.sst                # Sorted string tables (key → value)
├── *.log                # Write-ahead logs (rotated)
├── OPTIONS              # RocksDB options file
└── wal/                 # Write-ahead logs directory
    └── *.log
```

### Configuration

```rust
struct StoreConfig {
    /// Path to the database directory
    db_path: PathBuf,
    
    /// Maximum size of the memtable before flush (default: 64MB)
    write_buffer_size: usize,
    
    /// Maximum number of memtables (default: 4)
    max_write_buffer_number: usize,
    
    /// Minimum number of write buffers to merge (default: 2)
    min_write_buffer_number_to_merge: usize,
    
    /// Maximum number of background threads (default: 4)
    max_background_jobs: i32,
    
    /// Enable/disable WAL (default: true)
    enable_wal: bool,
    
    /// Enable/disable compression (default: true, snappy)
    enable_compression: bool,
    
    /// Maximum file size for SST files (default: 64MB)
    max_file_size: u64,
}
```

---

## Path Resolution

### Algorithm

```rust
fn resolve_path(auil_path: &str, current_tree_id: Option<&str>) -> ResolvedPath {
    // If path starts with a namespace prefix explicitly: return as-is
    if auil_path.starts_with("ui.") || auil_path.starts_with("task.") || 
       auil_path.starts_with("prefs.") || auil_path.starts_with("perm.") {
        return ResolvedPath::Explicit(auil_path.to_string());
    }
    
    // Otherwise: try in order
    // 1. If current_tree_id exists, try `ui.{tree_id}.{path}`
    if let Some(tree) = current_tree_id {
        let candidate = format!("ui.{}.{}", tree, auil_path);
        if store.exists(&candidate) {
            return ResolvedPath::UI(candidate);
        }
    }
    
    // 2. Try `task.{path}`
    let candidate = format!("task.{}", auil_path);
    if store.exists(&candidate) {
        return ResolvedPath::Task(candidate);
    }
    
    // 3. Try `prefs.{path}`
    let candidate = format!("prefs.{}", auil_path);
    if store.exists(&candidate) {
        return ResolvedPath::Prefs(candidate);
    }
    
    // 4. Default to `task.{path}` (creates it)
    ResolvedPath::Task(format!("task.{}", auil_path))
}
```

### Path Validation

All paths must:
1. Be valid UTF-8
2. Not exceed 1024 bytes
3. Not contain null bytes
4. Not start or end with a dot (`.`) — except for namespace prefixes
5. Not contain consecutive dots (`..`)
6. Only use alphanumeric characters, dots (`.`), underscores (`_`), and hyphens (`-`)

---

## Patch Protocol

### Patch Operations

The State Store supports five patch operations:

| Operation | Syntax | Description |
|-----------|--------|-------------|
| Update | `~id(props)` | Update properties of node `id` |
| Insert | `+anchor: node` | Insert `node` at `anchor` position |
| Remove | `-id` | Remove node `id` and its descendants |
| Replace | `!id: node` | Replace subtree at `id` with `node` |
| Move | `@id → other-id` | Move subtree from `id` to `other-id` |

### Patch Transaction

`state.patch(ops)` executes as a single atomic transaction:

1. **Acquire write lock** on the entire store (short-lived, <1ms typical)
2. **Validate** each op:
   - Check caller has `CAP_STATE_WRITE` for the path
   - Validate path syntax
   - For moves: validate both source and destination exist
3. **Apply** each op:
   - For `ui.<tree>` moves: update parent-child relationship atomically
   - For all other ops: write the new value, record the old value
4. **Increment** the global revision counter
5. **Write** the batch to RocksDB
6. **After fsync**, notify all `state.watch` subscribers

### Example Patch

```json
{
  "ops": [
    {"op": "~", "id": "ui.root.controls.play", "props": {"text": "Pause", "disabled": false}},
    {"op": "+", "anchor": "ui.root.controls.right_of_play", "node": {"kind": "button", "id": "stop", "text": "Stop"}},
    {"op": "-", "id": "ui.root.controls.old_button"}
  ]
}
```

---

## Subscriptions

### Subscription Structure

```rust
struct Subscription {
    /// Unique identifier for this subscription
    id: Uuid,
    
    /// Path prefix to watch (e.g., "ui.root.controls", "task.functions.*")
    path_prefix: String,
    
    /// Only deliver events with revision > since_revision
    since_revision: u64,
    
    /// The subscriber's MCP connection ID
    subscriber: MCPConnectionId,
    
    /// Last revision sent to this subscriber (for deduplication)
    last_sent_revision: u64,
    
    /// Created timestamp
    created_at: DateTime,
}
```

### Subscription Delivery

On each patch commit:

1. Find all subscriptions where `path_prefix` is a prefix of any changed path
2. For each matched subscription:
   - Construct a `PatchEvent`:
     ```rust
     struct PatchEvent {
         path: String,
         old_value: Option<Value>,
         new_value: Option<Value>,
         revision: u64,
         timestamp: DateTime,
     }
     ```
   - Send the event over the subscriber's MCP stream
3. If the subscriber's send buffer is full:
   - Drop the oldest undelivered event for that subscriber
   - Increment `dropped_events` counter (visible via `state.stats()`)

### Backpressure

- **Send buffer per subscriber:** 1024 events
- **Overflow policy:** Drop oldest event (FIFO)
- **Dropped events counter:** Tracked per subscriber, visible via `state.stats()`

---

## MCP Interface

### Methods

#### `state.get(path: string) → Value`

Get the value at the specified path.

**Parameters:**
- `path` — the path to read (e.g., `"ui.root.controls.play.text"`)

**Returns:**
- The value at the path, or `null` if the path does not exist

**Errors:**
- `E_PERMISSION_DENIED` — caller lacks `CAP_STATE_READ` for the path
- `E_INVALID_PATH` — path syntax is invalid

**Example:**
```json
// Request
{"method": "state.get", "params": {"path": "ui.root.title"}}

// Response
{"result": {"value": "Welcome to The Machine"}}
```

#### `state.set(path: string, value: Value) → {}`

Set the value at the specified path.

**Parameters:**
- `path` — the path to write
- `value` — the value to write (must be JSON-serializable)

**Errors:**
- `E_PERMISSION_DENIED` — caller lacks `CAP_STATE_WRITE` for the path
- `E_INVALID_PATH` — path syntax is invalid
- `E_READ_ONLY` — path is in a read-only namespace (e.g., `perm.*`)

**Example:**
```json
// Request
{"method": "state.set", "params": {"path": "prefs.theme", "value": "dark"}}

// Response
{}
```

#### `state.patch(ops: PatchOp[]) → {revision: u64}`

Apply a batch of patch operations atomically.

**Parameters:**
- `ops` — array of patch operations

**Returns:**
- `revision` — the new global revision number after the patch

**Errors:**
- `E_PERMISSION_DENIED` — caller lacks `CAP_STATE_WRITE` for one or more paths
- `E_INVALID_OP` — one or more operations are invalid
- `E_CONFLICT` — operations conflict (e.g., insert at non-existent anchor)

**Example:**
```json
// Request
{"method": "state.patch", "params": {"ops": [
  {"op": "~", "id": "ui.root.title", "props": {"text": "New Title"}},
  {"op": "+", "anchor": "ui.root.children_end", "node": {"kind": "text", "text": "Hello"}}
]}}

// Response
{"result": {"revision": 42}}
```

#### `state.watch(path_prefix: string, since_revision: u64) → Stream<PatchEvent>`

Subscribe to changes matching the path prefix.

**Parameters:**
- `path_prefix` — the prefix to watch (e.g., `"ui.root.controls"`)
- `since_revision` — only deliver events with revision > this value

**Returns:**
- A stream of `PatchEvent` objects

**Errors:**
- `E_PERMISSION_DENIED` — caller lacks `CAP_STATE_READ` for the prefix
- `E_INVALID_PREFIX` — prefix syntax is invalid

**Example:**
```json
// Request
{"method": "state.watch", "params": {"path_prefix": "ui.root", "since_revision": 0}}

// Stream of responses
{"result": {"path": "ui.root.title", "old_value": null, "new_value": "Hello", "revision": 1, "timestamp": "..."}}
{"result": {"path": "ui.root.controls.play", "old_value": {"text": "Play"}, "new_value": {"text": "Pause"}, "revision": 2, "timestamp": "..."}}
```

#### `state.stats() → Stats`

Get store statistics.

**Returns:**
```json
{
  "revision": 42,
  "total_keys": 1234,
  "total_subscriptions": 5,
  "dropped_events": {
    "subscriber_1": 0,
    "subscriber_2": 3
  },
  "storage_size": 1048576,
  "uptime": 3600.5
}
```

---

## Capability Gating

Every read or write is checked against the caller's manifest:

| Capability | Description |
|------------|-------------|
| `CAP_STATE_READ(paths=[...])` | Allowed to read from specified path prefixes |
| `CAP_STATE_WRITE(paths=[...])` | Allowed to write to specified path prefixes |

**Enforcement:**
- Checked **before** the call is served
- For `state.patch`, checked per-op
- For `state.watch`, checked at subscription time

**Example manifest:**
```json
{
  "capabilities": [
    {"cap": "CAP_STATE_READ", "paths": ["ui.root.*", "task.functions.*"]},
    {"cap": "CAP_STATE_WRITE", "paths": ["ui.root.controls.*", "task.intents.*"]}
  ]
}
```

---

## Recovery

### On Startup

1. Open the RocksDB database
2. If the manifest is corrupted:
   - Attempt to recover from the last valid SST file set
   - If recovery fails, initialize a fresh database
3. Read the global revision counter (`__revision`)
4. If the revision counter is missing (fresh install):
   - Initialize it to 0
   - Write a default `ui.welcome` tree
5. Subscriptions are **not** replayed on recovery — they are re-established by clients after reconnect

### Snapshot and Compaction

- **Snapshot:** RocksDB automatically creates snapshots during compaction
- **Compaction:** Background compaction merges SST files to reduce read amplification
- **Manual snapshot:** `state.snapshot()` MCP method creates a named snapshot

---

## Performance

### Targets

| Metric | Target |
|--------|--------|
| Read latency (p99) | < 500μs |
| Write latency (p99) | < 1ms |
| Patch latency (p99, 10 ops) | < 2ms |
| Subscription delivery latency | < 100μs |
| Memory usage | < 100MB |

### Optimizations

1. **In-memory cache:** Hot keys (frequently accessed UI paths) are cached in memory
2. **Batch writes:** Multiple patch ops are batched into a single RocksDB write
3. **Prefix compression:** RocksDB uses prefix compression for keys with common prefixes
4. **Bloom filters:** RocksDB uses bloom filters to avoid unnecessary disk reads

---

## See Also

- [Agent Core](../agent-core.md) — for the primary consumer of the State Store
- [UI Runtime](../ui-runtime.md) — for the UI tree rendering
- [Policy Broker](../policy-broker.md) — for capability enforcement
- [Event Bus](../event-bus.md) — for subscriptions and reactive behavior
