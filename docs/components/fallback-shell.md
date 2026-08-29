# Fallback Shell

**Layer:** L5  
**Type:** Deterministic, zero-inference, recovery UI  
**Language:** Rust or C  
**Dependencies:** State Store (for frozen view), MCP Bus (for recovery actions), Compositor  

---

## Overview

The Fallback Shell is a **minimal, fully deterministic UI and control layer** that works with **zero agent involvement**. It is the system's safety net — usable when inference is unavailable, before the local model loads, or when the Agent Core crashes.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Fallback Shell                                                │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Activation Manager                                        │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Boot        │  │ Runtime     │  │ Key Combo           │ │ │
│  │  │ Activation  │  │ Activation  │  │ Activation          │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  State Store Client                                        │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Read        │  │ Read        │  │ Fallback           │ │ │
│  │  │ last-good   │  │ revision    │  │ if no state        │ │ │
│  │  │ UI tree     │  │             │  │                     │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Frozen View Renderer (minimal UI subset)                  │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Text        │  │ Stack/Grid  │  │ Placeholder         │ │ │
│  │  │ Rendering   │  │ Layout      │  │ for unknown nodes   │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Recovery Console                                           │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Menu        │  │ Status      │  │ Action              │ │ │
│  │  │ Rendering   │  │ Display     │  │ Execution           │ │ │
│  │  │             │  │             │  │ (via MCP)           │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  MCP Client (for recovery actions)                        │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Design Goals

1. **Zero inference, not "less" inference** — no model in this component's path
2. **Boots before, and outlives, everything above L2** — usable before local model loads
3. **Read the truth, don't reconstruct it** — trusts the State Store's "last known-good" definition
4. **Still a citizen of the security model** — goes through Policy Broker like everything else

---

## Activation

### Activation Reasons

```rust
enum ActivationReason {
    /// Boot-time: before Agent Core readiness signal
    BootBeforeAgentReady,
    
    /// Runtime: UI Runtime crash
    UiRuntimeCrash,
    
    /// Runtime: Agent Core unreachable beyond grace period (5s)
    AgentUnreachable,
    
    /// Explicit: Ctrl+Alt+F9 key combination (captured by System Daemon)
    SafeModeKeyCombo,
    
    /// Runtime: Resource exhaustion (OOM affecting agent-core or local-model)
    ResourceExhaustion,
}
```

### Activation Flow

1. **Reason triggers** — one of the above conditions occurs
2. **Compositor takeover** — Fallback Shell takes over the primary surface
3. **State check** — If reason is `BootBeforeAgentReady` or `UiRuntimeCrash`:
   - Attempt to read the last committed `ui.<tree>` from the State Store
   - If successful: render the frozen view
   - If not: show the recovery console
4. **Recovery console** — If reason is `SafeModeKeyCombo` or `ResourceExhaustion`:
   - Show the recovery console directly

---

## Frozen View Rendering

### Simplified AST

The Frozen View renderer understands a simplified AST:

```rust
enum FrozenNode {
    /// A text node
    Text { content: String, x: i32, y: i32, size: i32 },
    
    /// A stack container
    Stack { children: Vec<FrozenNode>, direction: Direction, spacing: i32 },
    
    /// A grid container
    Grid { children: Vec<Vec<FrozenNode>>, gap: i32 },
    
    /// A placeholder for unknown nodes
    Placeholder { label: String },
}
```

### Translation

The renderer maps the AUIL tree to this AST by:

1. Ignoring non-text/non-layout nodes (buttons, media surfaces, etc.)
2. Replacing them with `Placeholder { label: node.label }`
3. Translating `stack`/`grid` containers to the equivalent frozen layout
4. Applying a fixed, system-default theme (not the user's theme)

### Theme

The Frozen View uses a **fixed, system-default theme**:

| Color | Value |
|-------|-------|
| Background | `#1a1a1a` |
| Foreground | `#ffffff` |
| Accent | `#007aff` |
| Banner background | `#ff0000` |
| Banner foreground | `#ffffff` |

### Banner

The banner "⚠️ Agent Unavailable" is rendered as a top-level overlay:
- z-index above everything
- Fixed red/white color scheme
- Positioned at the top of the screen

---

## Recovery Console

### Menu Layout

```
┌──────────────────────────────────────────────┐
│  ⚠ THE MACHINE — RECOVERY MODE               │
│  Agent is unavailable.                        │
│                                               │
│  [1] View system status                       │
│  [2] View logs                                │
│  [3] Restart agent (requires confirmation)    │
│  [4] Connect to network                       │
│  [5] Safe terminal (requires confirmation)    │
│                                               │
│  Press number or use arrow keys.              │
└──────────────────────────────────────────────┘
```

### Actions

| Action | MCP Method | Requires Confirmation |
|--------|------------|----------------------|
| View system status | `power.get_profile`, `display.get_modes`, `net.list_interfaces` | No |
| View logs | `policy.audit_query` | No |
| Restart agent | `systemd.restart agent-core` | Yes (protected unit) |
| Connect to network | `net.connect_wifi` | Yes (if not pre-authorized) |
| Safe terminal | `system.terminal` | Yes |

### Action Execution

Each action is executed via a direct MCP call:

```rust
fn execute_action(action: RecoveryAction) -> Result<(), ExecutionError> {
    match action {
        RecoveryAction::ViewStatus => {
            let profile = mcp_client.call("power.get_profile", params!());
            let modes = mcp_client.call("display.get_modes", params!());
            let net = mcp_client.call("net.list_interfaces", params!());
            display_status(profile, modes, net);
        }
        RecoveryAction::ViewLogs => {
            let logs = mcp_client.call("policy.audit_query", params!(query));
            display_logs(logs);
        }
        RecoveryAction::RestartAgent => {
            // This will go through policy.check and return CONFIRM
            mcp_client.call("systemd.restart", params!(unit: "agent-core"));
        }
        RecoveryAction::ConnectNetwork => {
            // This will go through policy.check and return CONFIRM if not pre-authorized
            mcp_client.call("net.connect_wifi", params!(ssid, credential_ref));
        }
        RecoveryAction::SafeTerminal => {
            // This will go through policy.check and return CONFIRM
            mcp_client.call("system.terminal", params!());
        }
    }
    Ok(())
}
```

### Input Handling

Input handling is keyboard-only:

1. **Number keys (1-5)** — select the corresponding action
2. **Arrow keys** — navigate the menu
3. **Enter** — execute the selected action
4. **Escape** — go back to the main menu
5. **Ctrl+C** — exit recovery console (only if Agent Core is available)

---

## State Store Integration

### Reading Last-Good State

The Fallback Shell reads the "last-known-good" state from the State Store:

```rust
fn read_last_good_state() -> Option<UiTree> {
    // 1. Read the revision counter
    let revision = state_store.get("__revision");
    
    // 2. Read the last committed UI tree
    let tree = state_store.get("ui.root");
    
    // 3. Fallback to a default welcome tree if none exists
    tree.or_else(|| default_welcome_tree())
}
```

### Default Welcome Tree

If no state exists, the Fallback Shell renders a default welcome tree:

```json
{
  "kind": "container",
  "children": [
    {"kind": "text", "props": {"content": "Welcome to The Machine", "size": 24, "alignment": "center"}},
    {"kind": "text", "props": {"content": "Agent is starting...", "size": 14, "alignment": "center"}}
  ]
}
```

---

## Dependencies

The Fallback Shell has **minimal dependencies**:

| Dependency | Purpose |
|------------|---------|
| `libc` | System calls |
| `wl_display` | Wayland communication |
| `xdg_shell` | Wayland shell protocol |
| `state_store_client` | Reading the State Store (raw socket) |
| `mcp_client` | Recovery actions (via MCP Bus) |

**No:**
- No LLM
- No Agent Core
- No UI Runtime
- No Policy Broker (except for recovery actions)
- No AUIL/ASL parser
- No dynamic linking except libc

---

## Performance

### Targets

| Metric | Target |
|--------|--------|
| Activation latency (boot) | < 100ms |
| Activation latency (runtime) | < 10ms |
| Render latency (60 fps) | < 16ms |
| Memory usage | < 10MB |

---

## Security Considerations

1. **Zero inference** — no LLM, no probabilistic code paths
2. **Still through Policy Broker** — recovery actions go through `policy.check`
3. **No special privileges** — the Fallback Shell is not a bypass
4. **Physical-only input** — keyboard input is the only input method
5. **Protected units** — restarting load-bearing units requires confirmation

---

## See Also

- [State Store](../state-store.md) — for reading last-good state
- [Agent Core](../agent-core.md) — for what triggers the fallback
- [System Daemon](../system-daemon.md) — for input provenance and safe mode key combo
- [Policy Broker](../policy-broker.md) — for recovery action enforcement
- [MCP Bus](../mcp-bus.md) — for recovery action execution
