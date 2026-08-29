# Lambda Server

**Layer:** L1  
**Type:** Deterministic (server) + Sandboxed (lambdas)  
**Language:** Rust (server), any (lambdas)  
**Dependencies:** Policy Broker, State Store, MCP Bus  

---

## Overview

The Lambda Server is a **local (with optional cloud burst) serverless runtime**. The agent deploys, updates, and invokes small sandboxed functions here to accomplish user tasks. Functions are orchestration code calling into a **vetted base image** — the agent is not allowed to hand-roll security-critical primitives.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Lambda Server                                                   │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Function Registry (State Store)                            │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ video_      │  │ download_   │  │ media_              │ │ │
│  │  │ player_v3   │  │ notifier_v1 │  │ player_v2           │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Process Supervisor                                             │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Warm Pool  │  │ Cold Start  │  │ Crash Loop          │ │ │
│  │  │ (persistent)│  │ (ephemeral) │  │ Detector            │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Sandbox Manager                                              │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ OCI         │  │ MicroVM     │  │ Seccomp             │ │ │
│  │  │ Containers  │  │ (Firecracker)│  │ Profiles            │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  MCP Interface: lambda.register, lambda.invoke, lambda.search │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Function Model

### Function Record

```rust
struct FunctionRecord {
    /// Unique name for this function (e.g., "video_player")
    name: String,
    
    /// Monotonically increasing version number
    version: u64,
    
    /// The function's manifest (capabilities, entrypoint, etc.)
    manifest: LambdaManifest,
    
    /// SHA-256 hash of the function's artifact (OCI image or WASM blob)
    artifact_hash: String,
    
    /// Current status of this function
    status: FunctionStatus,
    
    /// When this function was created
    created_at: DateTime,
    
    /// When this function was last invoked
    last_invoked: Option<DateTime>,
    
    /// Health check URL (for HTTP-based health checks)
    health_check_url: Option<String>,
    
    /// Health check interval
    health_check_interval: Option<Duration>,
}

enum FunctionStatus {
    /// Function is registered but not yet deployed
    Pending,
    
    /// Function is deployed and ready
    Ready,
    
    /// Function is running (for persistent functions)
    Running,
    
    /// Function is in a degraded state (crash-loop, health check failing)
    Degraded,
    
    /// Function has been explicitly disabled
    Disabled,
}
```

### Lambda Manifest

```rust
struct LambdaManifest {
    /// Human-readable description of what this function does
    description: String,
    
    /// Required capabilities
    capabilities: Vec<Capability>,
    
    /// MCP methods this function exposes (for mcp-intent registration)
    exposes_mcp: Vec<MethodEntry>,
    
    /// Event patterns this function handles (for event-handler registration)
    handles_event: Vec<EventPattern>,
    
    /// Base image to use (e.g., "ffmpeg:5.0", "python:3.11-slim")
    base_image: String,
    
    /// Entrypoint command
    entrypoint: String,
    
    /// Environment variables
    env: HashMap<String, String>,
    
    /// Resource limits
    resources: ResourceLimits,
    
    /// If true, this function should run as a persistent process
    persistent: bool,
    
    /// Health check configuration
    health: Option<HealthConfig>,
    
    /// Maximum number of concurrent invocations (for non-persistent)
    max_concurrency: Option<usize>,
}

struct ResourceLimits {
    /// Maximum CPU shares (relative weight)
    cpu_shares: Option<u64>,
    
    /// Maximum memory in bytes
    memory_limit: Option<u64>,
    
    /// Maximum execution time for a single invocation
    timeout: Option<Duration>,
    
    /// Maximum number of file descriptors
    max_fds: Option<u64>,
    
    /// GPU access (if true, requires CAP_GPU)
    gpu: bool,
    
    /// Network access (if true, requires CAP_NET_OUT)
    network: bool,
}

struct HealthConfig {
    /// Health check command (for command-based checks)
    command: Option<String>,
    
    /// Health check URL (for HTTP-based checks)
    url: Option<String>,
    
    /// Health check interval
    interval: Duration,
    
    /// Number of consecutive failures before marking as degraded
    failure_threshold: u32,
    
    /// Number of consecutive successes before marking as healthy
    success_threshold: u32,
}
```

### Capability Model

Each lambda declares required capabilities in its manifest:

| Capability | Description | Example |
|------------|-------------|---------|
| `CAP_NET_OUT(domains=[...])` | Outbound network access to specific domains | `CAP_NET_OUT(domains=["youtube.com", "api.github.com"])` |
| `CAP_FS_READ(paths=[...])` | Read access to specific filesystem paths | `CAP_FS_READ(paths=["/home/user/Music/*"])` |
| `CAP_FS_WRITE(paths=[...])` | Write access to specific filesystem paths | `CAP_FS_WRITE(paths=["/home/user/Downloads/*"])` |
| `CAP_GPU` | GPU access | `CAP_GPU` |
| `CAP_MIC` | Microphone access | `CAP_MIC` |
| `CAP_CAMERA` | Camera access | `CAP_CAMERA` |
| `CAP_IPC_CALL(targets=[...])` | IPC call access to specific targets | `CAP_IPC_CALL(targets=["mcp-bus", "state-store"])` |
| `CAP_STATE_READ(paths=[...])` | State Store read access | `CAP_STATE_READ(paths=["ui.root.*", "task.functions.*"])` |
| `CAP_STATE_WRITE(paths=[...])` | State Store write access | `CAP_STATE_WRITE(paths=["ui.root.controls.*"])` |
| `CAP_TIMER` | Timer/scheduler access | `CAP_TIMER(recurrence="@every 1h")` |
| `CAP_PURE` | No side effects (pure function) | `CAP_PURE` |

**Grant behavior:**
- Grants are **monotonic** — once granted, they cannot be narrowed
- Grants are **non-escalating** — a fresh `policy.check` is required for any broadening
- Grants are **per-identity** — each lambda has its own grant set

---

## Sandbox Execution

### Container Creation

The Lambda Server uses OCI containers (via `libcontainer` or `containerd`) with the following isolation:

1. **Namespaces:**
   - `pid` — separate process tree
   - `net` — separate network stack (unless `CAP_NET_OUT` granted)
   - `ipc` — separate IPC namespace
   - `uts` — separate hostname
   - `mount` — separate filesystem mount points

2. **Seccomp profile:**
   - Allow only syscalls in a pre-approved list
   - Read-only for most syscalls
   - `write` only to allowed paths (from `CAP_FS_WRITE`)
   - `open` only for allowed paths (from `CAP_FS_READ`/`CAP_FS_WRITE`)

3. **Capabilities:**
   - Drop all Linux capabilities
   - Add back only those explicitly in the manifest
   - Example: `CAP_NET_BIND_SERVICE` if `CAP_NET_OUT` and port binding requested

4. **Filesystem:**
   - Read-only root filesystem
   - Writable overlay for `/tmp`
   - Allowed paths from `CAP_FS_READ`/`CAP_FS_WRITE` are bind-mounted

5. **GPU access:**
   - Mediated via `virtio-gpu` with an allow-list of Vulkan/OpenGL commands
   - Only if `CAP_GPU` is granted

### MicroVM Alternative

For stronger isolation, the Lambda Server can use Firecracker microVMs:

- **Pros:** Stronger isolation, smaller attack surface
- **Cons:** Higher overhead, more complex setup
- **Use case:** High-security lambdas (e.g., handling sensitive data)

### Invocation Flow

1. **Validation** (Policy Broker): `policy.check` on the manifest
   - If `ALLOW`, returns a grant token
   - If `DENY` or `CONFIRM`, the invocation fails

2. **Image pull:**
   - Pull the base image from a local cache or registry
   - If cloud burst enabled, pull from a remote registry

3. **Container creation:**
   - Create the container with the specified isolation
   - Set up the filesystem with allowed paths
   - Configure network access (if `CAP_NET_OUT`)

4. **Warm pool:**
   - If the function is marked `persistent: true`, the container stays running
   - Otherwise, it's destroyed after the invocation returns

5. **Invocation:**
   - Send the payload over stdin or a Unix socket
   - Capture stdout/stderr
   - Set a timeout (from manifest or default 30s)

6. **Result:**
   - Parse the response (JSON or binary)
   - Return to caller

### Fast-Path Leases

When a UI component needs a low-latency channel to a lambda:

1. UI Runtime → MCP Bus → Lambda Server: `lambda.lease(name, streams=[...])`
2. Lambda Server → MCP Bus: returns a Unix socket path
3. UI Runtime connects directly to that socket
4. All subsequent calls on that socket bypass the MCP Bus entirely

The lease is time-limited (default 5 minutes) and can be renewed via `lambda.renew_lease`.

---

## Process Supervisor

### Warm Pool Management

The Process Supervisor maintains a pool of warm containers for persistent functions:

```rust
struct WarmPool {
    /// Map from function name to running container
    containers: HashMap<String, Container>,
    
    /// Map from function name to health status
    health: HashMap<String, HealthStatus>,
    
    /// Map from function name to last health check time
    last_health_check: HashMap<String, DateTime>,
}
```

**Health check loop:**
1. Every `health_check_interval` (default 10s), check each warm container
2. If health check fails, increment failure counter
3. If failure counter >= `failure_threshold`, mark as `Degraded`
4. If success counter >= `success_threshold`, mark as `Ready`

### Cold Start

For non-persistent functions, the Process Supervisor:
1. Creates a new container on demand
2. Destroys the container after the invocation completes
3. Caches the container image for faster subsequent starts

### Crash Loop Detection

1. On each crash, the Process Supervisor records the crash timestamp and increments a counter
2. If the counter exceeds `MAX_CRASHES_PER_MINUTE` (configurable, default 5):
   - Marks the function as `Degraded`
   - Emits an event: `Event { category: "health", pattern: "lambda.crash-loop", payload: { name, version, crash_count } }`
   - The Event Bus routes this to the Agent Core (if no local handler exists)
3. If the function is `persistent: true` and crashes, the Supervisor restarts it automatically

---

## Function Registry

### Storage

The function registry is stored in the State Store under `task.functions.*`:

```
task.functions.
├── video_player
│   ├── v1
│   │   ├── manifest
│   │   ├── artifact_hash
│   │   ├── status
│   │   └── created_at
│   ├── v2
│   │   ├── ...
│   └── current → v2  (symlink to current version)
├── download_notifier
│   └── v1
│       ├── ...
└── media_player
    └── v1
        ├── ...
```

### Registration Flow

1. Agent Core calls: `lambda.register(manifest, artifact)`
2. Lambda Server:
   - Validates the manifest (syntax, required fields)
   - Computes the artifact hash
   - Stores the function record in the State Store
   - Requests capability grants from the Policy Broker
3. Policy Broker:
   - Evaluates the manifest's capabilities against policy
   - Returns `ALLOW`, `DENY`, or `CONFIRM`
4. If `ALLOW`:
   - Lambda Server registers the function's MCP methods and event handlers
   - Returns success to Agent Core
5. If `DENY` or `CONFIRM`:
   - Lambda Server rolls back the registration
   - Returns error to Agent Core

### Versioning

- Each registration creates a new version
- Versions are immutable
- The `current` symlink points to the latest version
- Rollback is automatic if a function crash-loops or fails a health check

### Search

The Lambda Server supports searching for functions:

```rust
struct SearchQuery {
    /// Search in function names
    name: Option<String>,
    
    /// Search in function descriptions
    description: Option<String>,
    
    /// Filter by capabilities
    capabilities: Option<Vec<Capability>>,
    
    /// Filter by status
    status: Option<FunctionStatus>,
    
    /// Maximum number of results
    limit: Option<usize>,
}

struct SearchResult {
    functions: Vec<FunctionSummary>,
    total: usize,
}

struct FunctionSummary {
    name: String,
    version: u64,
    description: String,
    status: FunctionStatus,
    last_invoked: Option<DateTime>,
}
```

---

## MCP Interface

### Methods

#### `lambda.register(manifest: LambdaManifest, artifact: bytes) → {name: string, version: u64}`

Register a new function.

**Parameters:**
- `manifest` — the function's manifest
- `artifact` — the function's code (OCI image tar or WASM blob)

**Returns:**
- `name` — the function's name
- `version` — the version number

**Errors:**
- `E_INVALID_MANIFEST` — manifest syntax is invalid
- `E_PERMISSION_DENIED` — caller lacks permission to register functions
- `E_POLICY_DENIED` — Policy Broker denied the capability request
- `E_POLICY_CONFIRM_REQUIRED` — Policy Broker requires confirmation

**Example:**
```json
// Request
{"method": "lambda.register", "params": {
  "manifest": {
    "name": "video_player",
    "description": "Plays videos from various sources",
    "capabilities": ["CAP_NET_OUT(domains=[\"youtube.com\"])", "CAP_GPU"],
    "exposes_mcp": [{"name": "play", "description": "Play a video"}],
    "handles_event": [{"category": "input", "pattern": "video.play"}],
    "base_image": "ffmpeg:5.0",
    "entrypoint": "/app/play.sh",
    "persistent": true
  },
  "artifact": "<base64-encoded-tarball>"
}}

// Response
{"result": {"name": "video_player", "version": 1}}
```

#### `lambda.invoke(name: string, version: u64, payload: Value) → Value`

Invoke a function.

**Parameters:**
- `name` — the function's name
- `version` — the version to invoke (0 = latest)
- `payload` — the input payload (JSON-serializable)

**Returns:**
- The function's output (JSON-serializable)

**Errors:**
- `E_NOT_FOUND` — function not found
- `E_VERSION_NOT_FOUND` — version not found
- `E_DEGRADED` — function is in degraded state
- `E_TIMEOUT` — function invocation timed out
- `E_PERMISSION_DENIED` — caller lacks permission to invoke this function

**Example:**
```json
// Request
{"method": "lambda.invoke", "params": {
  "name": "video_player",
  "version": 0,
  "payload": {"url": "https://youtube.com/watch?v=dQw4w9WgXcQ"}
}}

// Response
{"result": {"status": "playing", "duration": 213}}
```

#### `lambda.search(query: SearchQuery) → SearchResult`

Search for functions.

**Parameters:**
- `query` — the search query

**Returns:**
- The search results

**Example:**
```json
// Request
{"method": "lambda.search", "params": {
  "query": {
    "description": "play",
    "limit": 5
  }
}}

// Response
{"result": {
  "functions": [
    {"name": "video_player", "version": 2, "description": "Plays videos...", "status": "Ready"},
    {"name": "audio_player", "version": 1, "description": "Plays audio...", "status": "Ready"}
  ],
  "total": 2
}}
```

#### `lambda.lease(name: string, streams: string[]) → {socket_path: string}`

Create a fast-path lease to a function.

**Parameters:**
- `name` — the function's name
- `streams` — the streams to lease (e.g., `["video", "audio"]`)

**Returns:**
- `socket_path` — the Unix socket path for direct communication

**Errors:**
- `E_NOT_FOUND` — function not found
- `E_NOT_PERSISTENT` — function is not persistent (leases require persistent functions)
- `E_PERMISSION_DENIED` — caller lacks permission

**Example:**
```json
// Request
{"method": "lambda.lease", "params": {
  "name": "video_player",
  "streams": ["video", "audio"]
}}

// Response
{"result": {"socket_path": "/run/the-machine/leases/lambda-video_player.sock"}}
```

#### `lambda.renew_lease(lease_id: string) → {}`

Renew a lease.

**Parameters:**
- `lease_id` — the lease to renew

**Example:**
```json
// Request
{"method": "lambda.renew_lease", "params": {"lease_id": "lease-1234"}}

// Response
{}
```

#### `lambda.status(name: string) → FunctionStatus`

Get a function's status.

**Parameters:**
- `name` — the function's name

**Returns:**
- The function's status

**Example:**
```json
// Request
{"method": "lambda.status", "params": {"name": "video_player"}}

// Response
{"result": {
  "name": "video_player",
  "version": 2,
  "status": "Ready",
  "last_invoked": "2024-01-15T10:30:00Z",
  "health": "healthy"
}}
```

#### `lambda.list() → FunctionSummary[]`

List all registered functions.

**Returns:**
- Array of function summaries

**Example:**
```json
// Request
{"method": "lambda.list"}

// Response
{"result": [
  {"name": "video_player", "version": 2, "status": "Ready"},
  {"name": "download_notifier", "version": 1, "status": "Ready"}
]}
```

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `THE_MACHINE_LAMBDA_DIR` | `/var/lib/the-machine/lambdas` | Directory for lambda artifacts |
| `THE_MACHINE_LAMBDA_CACHE_DIR` | `/var/cache/the-machine/lambdas` | Directory for cached base images |
| `THE_MACHINE_LAMBDA_TIMEOUT` | `30s` | Default invocation timeout |
| `THE_MACHINE_LAMBDA_MAX_CONCURRENCY` | `10` | Maximum concurrent invocations |
| `THE_MACHINE_LAMBDA_MAX_CRASHES_PER_MINUTE` | `5` | Crash loop threshold |

### Command-Line Arguments

```
lambda-server [OPTIONS]

Options:
  --artifact-dir <PATH>          Lambda artifacts directory
  --cache-dir <PATH>            Cache directory
  --timeout <DUR>               Default invocation timeout
  --max-concurrency <N>         Maximum concurrent invocations
  --max-crashes <N>            Crash loop threshold
  --use-microvm                Use Firecracker microVMs instead of OCI containers
  --help                       Show this help
```

---

## Performance

### Targets

| Metric | Target |
|--------|--------|
| Cold start latency (p99) | < 500ms |
| Warm invocation latency (p99) | < 50ms |
| Throughput (concurrent invocations) | 100+ |
| Memory overhead per container | < 10MB |

### Optimizations

1. **Warm pool** — persistent functions stay running
2. **Image caching** — base images are cached locally
3. **Fast-path leases** — direct sockets for high-throughput communication
4. **Lazy loading** — base images are only pulled when needed

---

## See Also

- [Policy Broker](./policy-broker.md) — for capability enforcement
- [State Store](./state-store.md) — for function registry storage
- [MCP Bus](./mcp-bus.md) — for method registration and resolution
- [Agent Core](./agent-core.md) — for the primary consumer
- [Event Bus](./event-bus.md) — for event handling
