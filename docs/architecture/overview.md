# Architecture Overview

The Machine is an agent-native OS built around a single principle: **the agent orchestrates, the system executes.**

---

## System Diagram

```
┌───────────────────────────────────────────────────────────────┐
│  L6  Human                                                     │
├───────────────────────────────────────────────────────────────┤
│  L5  UI Runtime (declarative renderer)  +  Wayland Compositor  │
├───────────────────────────────────────────────────────────────┤
│  L4  Agent Core (Hybrid LLM Router: local + cloud)             │
├───────────────────────────────────────────────────────────────┤
│  L3  MCP Bus (system‑wide protocol / message fabric)           │
├───────────────────────────────────────────────────────────────┤
│  L2  Policy Broker (capability & permission enforcement)       │
├───────────────────────────────────────────────────────────────┤
│  L1  Lambda Server (sandboxed function runtime)                │
│      + State Store + Event/Scheduler Bus                       │
├───────────────────────────────────────────────────────────────┤
│  L0  Kernel (Linux/BSD) + Drivers + I/O Subsystem              │
└───────────────────────────────────────────────────────────────┘
```

---

## Data Flow

### User Intent → Action

1. **User input** (text, voice, gesture) → Event Bus
2. Event Bus checks for registered handlers:
   - If a lambda handles it → deliver directly
   - Otherwise → wake the Agent Core
3. **Agent Core** reasons about the intent:
   - Local model handles routine tasks
   - Cloud model handles novel/complex tasks
4. Agent Core emits **MCP calls**:
   - `lambda.invoke` / `lambda.register`
   - `state.patch` (UI updates)
   - `policy.check` (capability requests)
5. **Policy Broker** evaluates each call:
   - `ALLOW` → grants a token, forwards
   - `DENY` → rejects with reason
   - `CONFIRM` → requires human approval
   - `HOLD` → queued for anomaly review
6. **Execution** happens at the appropriate layer:
   - Lambda Server runs sandboxed functions
   - State Store persists changes
   - UI Runtime renders the updated tree
   - System Daemon executes kernel operations

### Real-Time Path (No Inference)

```
Keyboard → System Daemon → Compositor → UI Runtime
```

This path never touches the Agent Core, Policy Broker, or any LLM. It delivers input events at native latency.

---

## Component Dependencies

```
Agent Core
    │
    ├── depends on → Policy Broker (for capability checks)
    ├── depends on → MCP Bus (for all communication)
    ├── depends on → Local Model (for Tier A reasoning)
    └── depends on → Cloud Model (for Tier B reasoning)

Policy Broker
    │
    ├── depends on → State Store (for policy storage, audit log)
    └── depends on → MCP Bus (for confirmation surface communication)

Lambda Server
    │
    ├── depends on → Policy Broker (for manifest validation)
    ├── depends on → State Store (for function registry)
    └── depends on → MCP Bus (for inter-component calls)

Event Bus
    │
    ├── depends on → State Store (for watch subscriptions)
    └── depends on → MCP Bus (for routing)

UI Runtime
    │
    ├── depends on → State Store (for UI tree)
    └── depends on → MCP Bus (for invoking lambdas)

Compositor
    │
    ├── depends on → System Daemon (for input events)
    └── depends on → UI Runtime (as primary client)

Fallback Shell
    │
    ├── depends on → State Store (for frozen view)
    └── depends on → MCP Bus (for recovery actions)

System Daemon
    │
    ├── depends on → Kernel (for syscalls, devices)
    └── depends on → Policy Broker (for grant token verification)
```

---

## Security Boundaries

| Boundary | Enforced By | What It Protects Against |
|----------|-------------|--------------------------|
| Agent → System | Policy Broker | Unauthorized capability use |
| Agent → Cloud | Local model (privacy tag) | Private data leaving the device |
| User → System | Confirmation Surface | Agent forging approvals |
| Malicious Lambda | Sandbox (OCI containers) | Escaping isolation |
| Prompt Injection | Broker (provenance check) | Agent acting on untrusted content |

---

## Boot Sequence

1. **Kernel** boots, **System Daemon** starts, drivers initialize
2. **Policy Broker** starts (minimal, deterministic — needs no inference)
3. **State Store** starts, loads last session's UI Tree
4. **MCP Bus** starts, loads registry from State Store
5. **Event Bus** starts
6. **Lambda Server** starts (warm pools initially empty)
7. **Compositor** starts
8. **UI Runtime** starts — renders whatever the State Tree holds
9. **Local model** loads (this is where boot may block)
10. **Agent Core** starts — once ready, it signals the Event Bus
11. **Fallback Shell** exits if everything above succeeded; otherwise it remains active

---

## Deployment Modes

| Mode | Description | When Used |
|------|-------------|-----------|
| **Normal** | All components running, agent online, local + cloud model available | Default |
| **Local-Only** | Agent Core refuses to escalate to cloud; local model only | User preference, offline, privacy-sensitive tasks |
| **Agent-Less** | Lambda Server, State Store, Event Bus, Compositor, UI Runtime continue without agent | Agent Core crash (until restart) |
| **Recovery** | Fallback Shell replaces UI Runtime; no inference anywhere | Before agent loads, after agent crash, safe mode key combo |
| **Degraded** | Local model available but cloud is unreachable; agent handles with reduced capability | Network outage, cloud API down |

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Keyboard → display latency | < 20ms (no inference in path) |
| Local model inference | < 100ms for routine tasks |
| Cloud model escalation | < 5s for first token |
| State Store write latency | < 1ms (p99) |
| Event Bus routing latency | < 500μs (p99) |
| Lambda cold start | < 500ms (container) |
| Lambda warm invocation | < 50ms (p99) |

---

## Next Steps

- Read the [Design Philosophy](./philosophy.md) for deeper context
- Explore the [Layer Reference](./layers.md) for detailed layer-by-layer specs
- Jump to a specific [Component](../components/) for implementation details
