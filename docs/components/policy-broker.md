# Policy Broker

**Layer:** L2  
**Type:** Deterministic, non-LLM, formally-scoped  
**Language:** Rust  
**Dependencies:** State Store (for policy storage, audit log), MCP Bus (for confirmation surface)  

---

## Overview

The Policy Broker is the **single most important safety component** in The Machine. It is a small, deterministic, formally-scoped service that mediates *everything* the Agent Core wants to do to the system. It enforces capability grants, validates schema, detects anomalies, and maintains an immutable audit log.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Policy Broker                                                   │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Policy Engine                                              │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Rule        │  │ Match       │  │ Decision            │ │ │
│  │  │ Evaluator   │  │ Expression  │  │ (ALLOW/DENY/        │ │ │
│  │  │             │  │ Compiler    │  │  CONFIRM/HOLD)      │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Anomaly Detector                                           │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Rate        │  │ Novel       │  │ Rapid               │ │ │
│  │  │ Limiter     │  │ Combo       │  │ Deploy              │ │ │
│  │  │             │  │ Detector    │  │ Detector            │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Confirmation Surface Daemon                                │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Template    │  │ Renderer    │  │ Input               │ │ │
│  │  │ Manager     │  │             │  │ Handler             │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Audit Log (State Store)                                     │ │
│  │  ┌─────────────────────────────────────────────────────────┐ │ │
│  │  │  perm.audit.*: [timestamp, caller, request, decision]   │ │ │
│  │  └─────────────────────────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  MCP Interface: policy.check, policy.register, policy.audit_query │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Policy Engine

### Rule Structure

```rust
struct PolicyRule {
    /// Unique identifier for this rule
    id: String,
    
    /// Human-readable description
    description: String,
    
    /// What this rule applies to (e.g., "all capabilities", "CAP_NET_OUT")
    applies_to: PolicySurface,
    
    /// The match expression (evaluated against the request)
    match_expr: MatchExpr,
    
    /// The decision for this rule
    decision: Decision,
    
    /// Priority (lower = higher priority)
    priority: u32,
    
    /// When this rule was created
    created_at: DateTime,
    
    /// When this rule was last modified
    modified_at: DateTime,
}

enum PolicySurface {
    /// Applies to all capability checks
    All,
    
    /// Applies to lambda capability manifests
    LambdaManifest,
    
    /// Applies to kernel operations
    KernelOp,
    
    /// Applies to UI patches
    UiPatch,
    
    /// Applies to intent-registry registrations
    IntentRegistration,
    
    /// Applies to a specific capability
    Capability(Capability),
}

enum Decision {
    ALLOW,
    DENY,
    CONFIRM,
    HOLD,
}
```

### Match Expression AST

```rust
enum MatchExpr {
    /// Always matches
    Always,
    
    /// Never matches
    Never,
    
    /// And
    And(Vec<MatchExpr>),
    
    /// Or
    Or(Vec<MatchExpr>),
    
    /// Not
    Not(Box<MatchExpr>),
    
    /// Equality: field == value
    Eq { field: String, value: Value },
    
    /// Set membership: field in {value1, value2, ...}
    In { field: String, values: Vec<Value> },
    
    /// Subset: field ⊆ {value1, value2, ...}
    Subset { field: String, set: Vec<Value> },
    
    /// Intersects: field ∩ {value1, value2, ...} != ∅
    Intersects { field: String, set: Vec<Value> },
    
    /// Regex match: field matches pattern
    Regex { field: String, pattern: String },
}
```

### Rule Evaluation

1. **Compile** — parse policy documents into AST
2. **Validate** — check for unreachable rules (static analysis)
3. **Evaluate** — for each request, evaluate match expressions in priority order
4. **Decision** — return the first matching decision, or `DENY` (default-deny)

**Example policy:**
```json
{
  "rules": [
    {
      "id": "default-deny",
      "description": "Deny everything by default",
      "applies_to": "All",
      "match_expr": "Always",
      "decision": "DENY",
      "priority": 0
    },
    {
      "id": "allow-state-read",
      "description": "Allow state read for ui paths",
      "applies_to": "Capability(CAP_STATE_READ)",
      "match_expr": {
        "Or": [
          {"Eq": {"field": "paths", "value": ["ui.root.*"]}},
          {"Eq": {"field": "paths", "value": ["ui.controls.*"]}}
        ]
      },
      "decision": "ALLOW",
      "priority": 10
    },
    {
      "id": "confirm-restart",
      "description": "Systemd restart requires confirmation",
      "applies_to": "KernelOp",
      "match_expr": {
        "Regex": {"field": "method", "pattern": "systemd\\.restart"}
      },
      "decision": "CONFIRM",
      "priority": 20
    }
  ]
}
```

### Default-Deny Enforcement

The policy engine **must** have a `default-deny` rule at the top of every policy. This is enforced by a static validation pass:

```rust
fn validate_policy(rules: &[PolicyRule]) -> Result<(), PolicyError> {
    // Check that at least one rule matches Always
    if !rules.iter().any(|r| matches!(r.match_expr, MatchExpr::Always)) {
        return Err(PolicyError::MissingDefaultDeny);
    }
    
    // Check that the first Always-rule is DENY
    let first_always = rules.iter()
        .find(|r| matches!(r.match_expr, MatchExpr::Always))
        .unwrap();
    if !matches!(first_always.decision, Decision::DENY) {
        return Err(PolicyError::DefaultDenyMustBeDeny);
    }
    
    // Check for unreachable rules
    // ...
    Ok(())
}
```

---

## Grant Tokens

### Token Structure

```rust
struct GrantToken {
    /// Unique identifier for this token
    token_id: Uuid,
    
    /// When this token was issued
    issued_at: DateTime,
    
    /// When this token expires
    expires_at: DateTime,
    
    /// The grant scope
    scope: GrantScope,
    
    /// Ed25519 signature over (token_id + expires_at + scope)
    signature: [u8; 64],
}

struct GrantScope {
    /// The MCP method that was granted
    method: String,
    
    /// Hash of the request payload (for replay prevention)
    request_hash: String,
    
    /// The requester's identity
    requester_identity: ComponentId,
}
```

### Token Lifecycle

1. **Issuance:** Policy Broker issues a token on `ALLOW`
2. **Verification:** Downstream components verify the token signature
3. **Expiration:** Tokens expire after a short time (default 5 minutes)
4. **Revocation:** Tokens can be revoked via `policy.revoke_token`

### Signing

The Broker uses an Ed25519 key pair for signing tokens:

- **Private key:** Generated at boot, stored in memory only
- **Public key:** Hard-coded in System Daemon and Lambda Server binaries

**Verification flow:**
1. Extract token from the MCP call
2. Verify signature using the public key
3. Verify that `method` matches the actual MCP method
4. Verify that `request_hash` matches the request payload
5. Verify that `expires_at` is in the future
6. Verify that `requester_identity` matches the caller

---

## Confirmation Surface

### Architecture

The Confirmation Surface is a **separate process** owned by the Broker that renders on a **compositor-protected surface**.

```
┌─────────────────────────────────────────────────────────────────┐
│  Policy Broker                                                 │
│                                                                   │
│  ┌─────────────┐    ┌─────────────────────────────────────────┐ │
│  │ Policy      │───▶│ Confirmation Surface Daemon              │ │
│  │ Engine      │    │                                           │ │
│  │             │    │  ┌─────────────┐  ┌───────────────────┐ │ │
│  │             │    │  │ Template    │  │ Compositor        │ │ │
│  │             │    │  │ Renderer    │  │ Connection        │ │ │
│  │             │    │  └─────────────┘  └───────────────────┘ │ │
│  └─────────────┘    │                                           │ │
│                     │  ┌─────────────┐  ┌───────────────────┐ │ │
│                     │  │ Input       │  │ Confirmation      │ │ │
│                     │  │ Handler     │  │ State Machine     │ │ │
│                     │  └─────────────┘  └───────────────────┘ │ │
│                     └─────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Template System

Templates are **hand-authored**, fixed, and stored in the Broker's binary. The agent cannot create or modify templates.

**Template structure:**
```json
{
  "name": "capability-grant-request",
  "description": "Request to grant a capability",
  "layout": [
    {"type": "header", "text": "⚠️ Capability Request"},
    {"type": "text", "text": "{{requester}} is requesting: {{description}}"},
    {"type": "text", "text": "Capability: {{capability}}"},
    {"type": "text", "text": "Scope: {{scope}}"},
    {"type": "button", "label": "{{confirm_label}}", "action": "confirm"},
    {"type": "button", "label": "Deny", "action": "deny"},
    {"type": "timer", "timeout": "{{timeout}}"}
  ],
  "placeholders": [
    {"name": "requester", "type": "string"},
    {"name": "description", "type": "string"},
    {"name": "capability", "type": "string"},
    {"name": "scope", "type": "string"},
    {"name": "confirm_label", "type": "string"},
    {"name": "timeout", "type": "string"}
  ]
}
```

### Randomization

To prevent automation, the confirmation dialog features:

1. **Random position:** The "yes" button is placed at one of three positions (top-left, center, bottom-right)
2. **Random label:** The "yes" button is labeled with one of three labels ("Confirm", "Yes", "Proceed")
3. **Random correlation ID:** Displayed in the dialog so the user knows which request they're confirming
4. **Timeout:** Dialog expires after 60 seconds (default)

### Input Provenance

The Confirmation Surface Daemon **only accepts input events with a valid ProvenanceMarker** (from the System Daemon). This ensures that:

1. The input event came from physical hardware (not a synthetic event)
2. The input event is from the current session (HMAC secret is boot-time)
3. The input event cannot be replayed (sequence number + timestamp)

### State Machine

```
┌─────────────────┐
│  IDLE           │
└────────┬────────┘
         │
         │ confirmation request received
         ▼
┌─────────────────┐
│  RENDERING      │ (render template, wait for input)
└────────┬────────┘
         │
         ├─ user confirms → send ALLOW
         ├─ user denies → send DENY
         ├─ timeout → send DENY
         └─ error → send DENY
         ▼
┌─────────────────┐
│  IDLE           │
└─────────────────┘
```

---

## Anomaly Detector

### Detectors

| Detector | Trigger | Action |
|----------|---------|--------|
| Rate limiter | > 100 `policy.check` calls/min from same identity | New requests go to `HOLD` for 60s |
| Novel capability combo | First time a lambda requests `CAP_FS_WRITE` + `CAP_CAMERA` together | `HOLD` until manual review |
| Rapid deploy | > 10 `lambda.register` calls/min from Agent Core | `HOLD` on new registrations for 5 min |
| Repeated DENY | > 20 `DENY` responses to the same identity in 5 min | `HOLD` on all future requests from that identity for 10 min |

### Hold Resolution

When a request is held, the Broker returns a `HOLD` with a `hold_id` and an estimated resolution time. The caller can poll `policy.hold_status(hold_id)` to check progress.

```rust
struct HoldStatus {
    hold_id: String,
    reason: String,
    status: HoldState,
    estimated_resolution: Option<DateTime>,
}

enum HoldState {
    /// The hold is still active
    Pending,
    
    /// The hold has been resolved (ALLOW, DENY, or CONFIRM)
    Resolved { decision: Decision },
    
    /// The hold timed out
    TimedOut,
}
```

### Configuration

```rust
struct AnomalyConfig {
    /// Rate limit: max policy.check calls per minute per identity
    rate_limit_per_minute: usize,
    
    /// Rate limit hold duration
    rate_limit_hold_duration: Duration,
    
    /// Rapid deploy threshold
    rapid_deploy_threshold: usize,
    
    /// Rapid deploy window (default: 1 minute)
    rapid_deploy_window: Duration,
    
    /// Rapid deploy hold duration
    rapid_deploy_hold_duration: Duration,
    
    /// Repeated DENY threshold
    repeated_deny_threshold: usize,
    
    /// Repeated DENY window (default: 5 minutes)
    repeated_deny_window: Duration,
    
    /// Repeated DENY hold duration
    repeated_deny_hold_duration: Duration,
}
```

---

## Audit Log

### Log Entry

```rust
struct AuditEntry {
    /// Unique identifier for this entry
    id: Uuid,
    
    /// When the event occurred
    timestamp: DateTime,
    
    /// Who made the request
    caller: ComponentId,
    
    /// What was requested
    request: AuditRequest,
    
    /// The decision
    decision: Decision,
    
    /// Why the decision was made
    reason: String,
    
    /// Which rule fired (if any)
    rule_id: Option<String>,
    
    /// The grant token issued (if ALLOW)
    token_id: Option<Uuid>,
    
    /// The anomaly hold ID (if HOLD)
    hold_id: Option<String>,
}

enum AuditRequest {
    /// A policy.check call
    PolicyCheck { method: String, request: Value },
    
    /// A policy.register call
    PolicyRegister { policy: Value },
    
    /// A token revocation
    TokenRevoke { token_id: Uuid },
}
```

### Storage

The audit log is stored in the State Store under `perm.audit.*`:

```
perm.audit.
├── 2024-01-15
│   ├── 10:30:45-uuid1
│   ├── 10:31:00-uuid2
│   └── ...
├── 2024-01-16
│   ├── ...
└── ...
```

**Query API:**
```rust
struct AuditQuery {
    /// Filter by timestamp range
    from: Option<DateTime>,
    to: Option<DateTime>,
    
    /// Filter by caller
    caller: Option<ComponentId>,
    
    /// Filter by decision
    decision: Option<Decision>,
    
    /// Filter by rule
    rule_id: Option<String>,
    
    /// Maximum number of results
    limit: Option<usize>,
    
    /// Offset for pagination
    offset: Option<usize>,
}
```

---

## MCP Interface

### Methods

#### `policy.check(method: string, request: Value, provenance: Option<Provenance>) → {decision: string, token: Option<string>, reason: string}`

Check if a request is allowed.

**Parameters:**
- `method` — the MCP method being requested
- `request` — the request payload
- `provenance` — optional provenance information (for prompt-injection containment)

**Returns:**
- `decision` — `ALLOW`, `DENY`, `CONFIRM`, or `HOLD`
- `token` — the grant token (if `ALLOW`)
- `reason` — why the decision was made

**Errors:**
- `E_INVALID_REQUEST` — request syntax is invalid
- `E_POLICY_INVALID` — policy is invalid

**Example:**
```json
// Request
{"method": "policy.check", "params": {
  "method": "lambda.register",
  "request": {
    "manifest": {
      "name": "video_player",
      "capabilities": ["CAP_GPU", "CAP_NET_OUT(domains=[\"youtube.com\"])"]
    }
  },
  "provenance": {"user_intent": "play video from YouTube"}
}}

// Response
{"result": {
  "decision": "ALLOW",
  "token": "eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCJ9...",
  "reason": "Rule 'allow-video-player' matched"
}}
```

#### `policy.register(policy: PolicyDocument) → {}`

Register or update a policy.

**Parameters:**
- `policy` — the policy document (JSON)

**Errors:**
- `E_INVALID_POLICY` — policy syntax is invalid
- `E_PERMISSION_DENIED` — caller lacks permission to register policies

**Example:**
```json
// Request
{"method": "policy.register", "params": {
  "policy": {
    "version": "1.0",
    "rules": [
      {"id": "default-deny", "match": "Always", "decision": "DENY"},
      {"id": "allow-state-read", "match": {...}, "decision": "ALLOW"}
    ]
  }
}}

// Response
{}
```

#### `policy.audit_query(query: AuditQuery) → AuditEntry[]`

Query the audit log.

**Parameters:**
- `query` — the audit query

**Returns:**
- Array of audit entries

**Example:**
```json
// Request
{"method": "policy.audit_query", "params": {
  "query": {
    "caller": "agent-core",
    "decision": "DENY",
    "limit": 10
  }
}}

// Response
{"result": [
  {"timestamp": "2024-01-15T10:30:45Z", "caller": "agent-core", "request": {...}, "decision": "DENY", "reason": "Default deny"}
]}
```

#### `policy.revoke_token(token_id: string) → {}`

Revoke a grant token.

**Parameters:**
- `token_id` — the token to revoke

**Example:**
```json
// Request
{"method": "policy.revoke_token", "params": {"token_id": "token-1234"}}

// Response
{}
```

#### `policy.hold_status(hold_id: string) → HoldStatus`

Get the status of a held request.

**Parameters:**
- `hold_id` — the hold to check

**Returns:**
- The hold status

**Example:**
```json
// Request
{"method": "policy.hold_status", "params": {"hold_id": "hold-1234"}}

// Response
{"result": {
  "status": "Pending",
  "reason": "Rate limit exceeded",
  "estimated_resolution": "2024-01-15T10:35:00Z"
}}
```

---

## Protected Units

`systemd.stop`/`restart`/`disable` on load-bearing units are hard-wired to `CONFIRM` — not policy-overridable.

**Protected units:**
- `system-daemon`
- `policy-broker`
- `state-store`
- `mcp-bus`
- `event-bus`
- `compositor`
- `ui-runtime`
- `agent-core`
- `lambda-server`

---

## Performance

### Targets

| Metric | Target |
|--------|--------|
| Policy evaluation latency (p99) | < 100μs |
| Token issuance latency | < 1ms |
| Confirmation surface render latency | < 50ms |
| Audit log write latency | < 1ms |

---

## Security Considerations

1. **Deny by default** — the Broker is the immune system, not the agent's judgment
2. **Prompt-injection containment** — ingested content is treated as data, never as instructions
3. **Provenance over content** — the Broker checks provenance, not semantics
4. **Formal gate-checkable** — the Broker is deterministic and not probabilistic
5. **Confirmation the agent cannot forge** — confirmation dialogs are authored and rendered by the Broker

---

## See Also

- [Lambda Server](../lambda-server.md) — for capability manifests
- [Agent Core](../agent-core.md) — for the primary consumer
- [MCP Bus](../mcp-bus.md) — for method registration and resolution
- [State Store](../state-store.md) — for policy and audit log storage
- [Compositor](../compositor.md) — for the confirmation surface
