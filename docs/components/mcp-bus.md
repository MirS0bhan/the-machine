# MCP Bus

**Layer:** L3  
**Type:** Deterministic, non-LLM  
**Language:** Rust  
**Dependencies:** State Store (for registry persistence — planned)  
**Implementation:** `mcp-bus/src/` — dynamic registry, pattern matching, `bus.resolve`, `bus.list_routes`, internal `_bus.register`

---

## Overview

The MCP Bus is the **uniform protocol connecting every layer** of The Machine. Not just "how the agent talks to tools" — MCP *is* the system bus. It is a thin router whose entire job is "given a method name, find the process that should handle it, forward the call, forward the response."

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  MCP Bus                                                          │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Registry (in-memory, backed by State Store)                │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ system-op   │  │ state-op    │  │ mcp-intent          │ │ │
│  │  │ (fixed)     │  │ (fixed)     │  │ (lambda registered) │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  │  ┌─────────────┐  ┌─────────────────────────────────────┐ │ │
│  │  │ event-      │  │ handler-identity → connection        │ │ │
│  │  │ handler     │  │                                      │ │ │
│  │  │ (registered)│  │                                      │ │ │
│  │  └─────────────┘  └─────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Router                                                       │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Method     │  │ Namespace   │  │ Forward to          │ │ │
│  │  │ Parsing    │  │ Extraction  │  │ Handler             │ │ │
│  │  │             │  │             │  │                     │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Connection Manager                                          │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Client      │  │ Multiplex   │  │ Fast-Path           │ │ │
│  │  │ Registry    │  │ (stream IDs)│  │ Leases              │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Unix Socket: /run/the-machine/mcp-bus.sock                  │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Wire Protocol

### Framing

Length-prefixed, similar to Cap'n Proto or gRPC:

```
+--------+--------+--------+--------+--------+--------+--------+
| magic  | version| flags  | length (u32)  | payload (bytes)    |
| 0x4D43 | 0x01   | 0x00   |               |                      |
+--------+--------+--------+--------+--------+--------+--------+
```

- `magic`: `0x4D4350` ("MCP") as 3 bytes, 1 byte reserved
- `version`: 1 byte, currently `0x01`
- `flags`: 1 byte, currently unused (reserved for future extensions)
- `length`: 32-bit big-endian length of the payload (not including header)
- `payload`: JSON or MessagePack serialized MCP call/response

### Message Structure

```rust
struct McpMessage {
    /// Unique identifier for this message
    id: Uuid,
    
    /// Stream ID (for multiplexing)
    stream_id: u64,
    
    /// Type of message
    type: MessageType,
    
    /// Method name (for requests)
    method: Option<String>,
    
    /// Parameters (for requests)
    params: Option<Value>,
    
    /// Result (for responses)
    result: Option<Value>,
    
    /// Error (for responses)
    error: Option<Error>,
}

enum MessageType {
    Request,
    Response,
    Notification,  // one-way, no response expected
    Stream,        // streaming data (for fast-path leases)
}

struct Error {
    code: String,
    message: String,
    details: Option<Value>,
}
```

### Connection Multiplexing

Each component opens a single Unix socket (`/run/the-machine/mcp-bus.sock`) and sends messages with a `stream_id` in the payload header. This allows a single connection to multiplex many concurrent RPCs and streams.

```rust
struct Connection {
    /// Unique identifier for this connection
    id: Uuid,
    
    /// Component identity
    component: ComponentId,
    
    /// Map from stream_id to pending request
    pending_requests: HashMap<u64, PendingRequest>,
    
    /// Active streams (for fast-path leases)
    active_streams: HashMap<u64, Stream>,
    
    /// Send queue
    send_queue: VecDeque<McpMessage>,
}
```

---

## Registry

### Registry Structure

The registry is the **single source of truth** for routing. It has four namespaces:

| Namespace | Description | Populated By |
|-----------|-------------|--------------|
| `system-op` | System Daemon operations | OS image (fixed) |
| `state-op` | State Store operations | OS image (fixed) |
| `mcp-intent` | Lambda MCP methods | Lambda Server (via `lambda.register`) |
| `event-handler` | Event handlers | Lambda Server (via `lambda.register`) |

### Registry Entry

```rust
struct RegistryEntry {
    /// The method or pattern key
    key: String,
    
    /// The namespace (system-op, state-op, mcp-intent, event-handler)
    namespace: RegistryNamespace,
    
    /// The component that handles this entry
    handler_identity: ComponentId,
    
    /// Reference to the manifest that registered this entry
    manifest_ref: Option<String>,
    
    /// When this entry was registered
    registered_at: DateTime,
    
    /// If true, this entry is immutable (cannot be unregistered)
    immutable: bool,
}

enum RegistryNamespace {
    SystemOp,      // system-* operations
    StateOp,       // state-* operations
    McpIntent,     // MCP method names
    EventHandler,  // event patterns
}
```

### Storage

The registry is stored in the State Store under `perm.mcp_routes.*`:

```
perm.mcp_routes.
├── mcp-intent
│   ├── video_player.play
│   ├── video_player.stop
│   ├── video_player.pause
│   └── download_notifier.notify
├── event-handler
│   ├── input.video.play
│   └── health.lambda.crash
├── system-op
│   ├── power.get_profile
│   ├── power.set_profile
│   └── ...
└── state-op
    ├── state.get
    ├── state.set
    ├── state.patch
    └── state.watch
```

### Resolution Algorithm

Given a method name, the resolution is **O(1)** lookup:

```rust
fn resolve_method(method: &str) -> ResolutionResult {
    // 1. Extract namespace from method prefix
    let prefix = method.split('.').next().unwrap_or("");
    
    let namespace = match prefix {
        "state" => RegistryNamespace::StateOp,
        "power" | "display" | "net" | "audio" | "system-daemon" => RegistryNamespace::SystemOp,
        _ => RegistryNamespace::McpIntent,  // fallback to mcp-intent
    };
    
    // 2. Look up the fully-qualified key
    let key = format!("{}.{}", namespace.to_key_prefix(), method);
    
    // 3. Return the handler or Agent Core fallback
    if let Some(entry) = registry.get(&key) {
        ResolutionResult::Handler(entry.handler_identity)
    } else if namespace == RegistryNamespace::McpIntent {
        // Fallback to Agent Core for unknown MCP intents
        ResolutionResult::AgentCore
    } else {
        // For system-op and state-op, missing means the system is broken
        ResolutionResult::Error("handler not found")
    }
}
```

### Registration Validation

The MCP Bus **does not** allow direct registration. Registration happens as a side effect of a Broker-validated `lambda.register` or `event.subscribe` call.

**Validation checks:**
1. **Exclusivity:** No other component can claim the same key
2. **Namespace validity:** The key must be in the correct format
3. **Broker validation:** The registration must have passed `policy.check`

---

## Fast-Path Leases

### Overview

For hot-loop IPC (e.g., UI Runtime ↔ media lambda), the bus can establish a **lease** that allows direct communication, bypassing the bus for subsequent calls. The resolution cost is paid once per lease.

### Lease Flow

1. **Request:** Caller sends: `bus.lease(method="player.stream", target="lambda:video_player")`
2. **Resolution:** Bus resolves the target using the registry
3. **Socket creation:** Bus creates a new Unix socket pair
4. **Delivery:** Bus sends one end to the caller, one end to the target
5. **Direct communication:** Both sides use the socket directly for subsequent calls
6. **Teardown:** Either side can close the socket early; lease expires after 5 minutes

### Lease Structure

```rust
struct Lease {
    /// Unique identifier for this lease
    lease_id: Uuid,
    
    /// The method this lease is for
    method: String,
    
    /// The target component
    target: ComponentId,
    
    /// The caller component
    caller: ComponentId,
    
    /// Unix socket path for the lease
    socket_path: String,
    
    /// When this lease was created
    created_at: DateTime,
    
    /// When this lease expires
    expires_at: DateTime,
    
    /// Active streams on this lease
    active_streams: Vec<Stream>,
}
```

### Lease Lifecycle

```
┌─────────────────┐
│  REQUESTED      │
└────────┬────────┘
         │
         │ socket created and delivered
         ▼
┌─────────────────┐
│  ACTIVE         │
└────────┬────────┘
         │
         ├─ both sides close → LEASED_ENDED
         ├─ timeout → EXPIRED
         └─ error → ABORTED
```

---

## MCP Interface

### Methods (Bus-specific)

#### `bus.resolve(method: string) → {handler, namespace, pattern}`

Look up a method in the registry. Implemented as `bus.resolve` (replaces draft `bus.registry.lookup`).

#### `bus.list_routes(namespace?: string) → RouteEntry[]`

List all entries in a namespace. Implemented as `bus.list_routes`.

#### `bus.lease(method: string, target: string) → {lease_id: string, socket_path: string}`

Create a fast-path lease.

**Parameters:**
- `method` — the method to lease
- `target` — the target component

**Returns:**
- `lease_id` — the lease ID
- `socket_path` — the Unix socket path for direct communication

**Example:**
```json
// Request
{"method": "bus.lease", "params": {"method": "video_player.stream", "target": "lambda:video_player"}}

// Response
{"result": {"lease_id": "lease-1234", "socket_path": "/run/the-machine/leases/lease-1234.sock"}}
```

#### `bus.lease.renew(lease_id: string) → {}`

Renew a lease.

**Parameters:**
- `lease_id` — the lease to renew

**Example:**
```json
// Request
{"method": "bus.lease.renew", "params": {"lease_id": "lease-1234"}}

// Response
{}
```

#### `bus.stats() → Stats`

Get bus statistics.

**Returns:**
```json
{
  "connections": 5,
  "pending_requests": 10,
  "active_streams": 3,
  "active_leases": 2,
  "registry_entries": 25,
  "messages_forwarded": 1000,
  "messages_dropped": 5,
  "uptime": 3600.5
}
```

---

## Error Handling

### Error Codes

| Code | Meaning |
|------|---------|
| `E_NOT_FOUND` | Method not found in registry |
| `E_LEASE_EXPIRED` | Lease has expired |
| `E_CONNECTION_CLOSED` | Connection closed |
| `E_MALFORMED_FRAME` | Malformed MCP frame |
| `E_INVALID_NAMESPACE` | Invalid registry namespace |
| `E_LEASE_EXISTS` | Lease already exists for this method/target |

### Retry Logic

The MCP Bus itself does **not** implement retries. Retries are handled by the caller.

---

## Performance

### Targets

| Metric | Target |
|--------|--------|
| Registry lookup latency (p99) | < 50μs |
| Message forwarding latency (p99) | < 100μs |
| Connection establishment latency | < 1ms |
| Lease creation latency | < 10ms |
| Memory usage | < 50MB |

### Optimizations

1. **O(1) lookup** — hash map for registry entries
2. **Zero-copy forwarding** — messages are forwarded without copying
3. **Connection pooling** — each component maintains a single connection
4. **Fast-path leases** — direct sockets for high-throughput communication

---

## See Also

- [Policy Broker](../policy-broker.md) — for capability enforcement
- [State Store](../state-store.md) — for registry persistence
- [Agent Core](../agent-core.md) — for the fallthrough consumer
- [Lambda Server](../lambda-server.md) — for method registration
- [Event Bus](../event-bus.md) — for event pattern registration
