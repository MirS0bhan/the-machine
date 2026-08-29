# Event/Scheduler Bus

**Layer:** L1  
**Type:** Deterministic, non-LLM  
**Language:** Rust  
**Dependencies:** State Store (for `state.watch` subscriptions)  

---

## Overview

The Event/Scheduler Bus is an **async event bus** that lets the system be reactive, not strictly turn-based. It decides *when* the Agent Core needs to be invoked at all. Most events (e.g., "video frame decoded, render it") are handled entirely inside L1/L0 without ever reaching the agent. Only events that require a *decision* get routed up to L4.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Event/Scheduler Bus                                               │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Event Ingress                                               │ │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────┐ │ │
│  │  │ MCP     │  │ State   │  │ System  │  │ Lambda          │ │ │
│  │  │ Calls   │  │ Watch   │  │ Daemon  │  │ Health Events   │ │ │
│  │  └────┬────┘  └────┬────┘  └────┬────┘  └────────┬────────┘ │ │
│  └───────┼─────────────┼─────────────┼─────────────────┼────────┘ │
│          │             │             │                 │          │
│          └─────────────┼─────────────┘                 │          │
│                            │                             │          │
│                            ▼                             ▼          │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Routing Table                                             │ │
│  │  (Category, Pattern) → HandlerEntry                        │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                            │                             │          │
│          ┌─────────────────┴─────────────────┐                 │          │
│          │                                   │                 │          │
│          ▼                                   ▼                 ▼          │
│  ┌─────────────┐              ┌─────────────┐         ┌─────────────┐ │
│  │  Subscribe  │              │  Direct     │         │  Agent      │ │
│  │  to State  │              │  to Lambda  │         │  Wake       │ │
│  │  Store     │              │             │         │             │ │
│  └─────────────┘              └─────────────┘         └─────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Scheduler (Timer Heap)                                    │ │
│  │  ┌─────────────────────────────────────────────────────────┐ │ │
│  │  │  ScheduledEvent { trigger_time, payload, recurring }     │ │ │
│  │  └─────────────────────────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  MCP Interface: event.emit, event.subscribe, event.schedule  │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Design Goals

1. **Most events never reach the Agent Core** — the default routing outcome is *local resolution*
2. **One mechanism, two jobs** — "event bus" and "scheduler" are the same component
3. **Routing is inspectable, not implicit** — can ask the bus *why* a given event class does or doesn't wake the agent
4. **Built on the State Store's primitives** — uses `state.watch` for subscriptions, not a separate event log

---

## Event Model

### Event Structure

```rust
struct Event {
    /// Unique identifier for this event
    id: Uuid,
    
    /// Event category (see below)
    category: Category,
    
    /// Event pattern (used for routing)
    pattern: String,
    
    /// The source of the event
    source: String,
    
    /// Event payload (JSON-serializable)
    payload: Value,
    
    /// Timestamp when the event was created
    timestamp: DateTime,
    
    /// State Store revision at the time of the event
    state_revision: u64,
    
    /// Flag: does this event require agent decision?
    requires_decision: bool,
    
    /// Flag: has this event been coalesced?
    coalesced: bool,
}
```

### Categories

| Category | Description | Source |
|----------|-------------|--------|
| `input` | User input events (keyboard, mouse, touch) | System Daemon |
| `health` | Component health events (crash, restart, heartbeat) | All components |
| `lambda` | Lambda execution events (start, stop, output) | Lambda Server |
| `state` | State Store change events | State Store |
| `timer` | Scheduled timer events | Scheduler |
| `external` | External events (notifications, sensors) | External sources |
| `system` | System-level events (power, network) | System Daemon |

### Patterns

Patterns are used for routing. They support:

- **Literal matching:** `"lambda.start"` matches only `"lambda.start"`
- **Wildcard matching:** `"lambda.*"` matches any lambda event
- **Prefix matching:** `"health.lambda.*"` matches any health event for lambdas
- **Suffix matching:** `"*.crash"` matches any crash event

---

## Routing

### Routing Table

```rust
struct RoutingTable {
    /// Map from (category, pattern) to handler
    entries: HashMap<(Category, String), HandlerEntry>,
    
    /// Map from subscriber ID to their subscriptions
    subscriptions: HashMap<SubscriberId, Vec<Subscription>>,
}

struct HandlerEntry {
    /// The component that handles this event
    handler_identity: ComponentId,
    
    /// When this handler was registered
    registered_at: DateTime,
    
    /// Reference to the manifest that registered this handler
    manifest_ref: String,
    
    /// Priority (higher = processed first)
    priority: u32,
}

struct Subscription {
    /// The subscriber's identity
    subscriber: ComponentId,
    
    /// The category to subscribe to
    category: Category,
    
    /// The pattern to match
    pattern: String,
    
    /// Only deliver events with revision > since_revision
    since_revision: u64,
}
```

### Routing Algorithm

```rust
fn route_event(event: Event) -> RoutingDecision {
    // 1. Check explicit handlers
    let key = (event.category.clone(), event.pattern.clone());
    if let Some(handler) = self.routing_table.entries.get(&key) {
        return RoutingDecision::Handler(handler.handler_identity.clone());
    }
    
    // 2. Check if event is flagged requires_decision
    if event.requires_decision {
        return RoutingDecision::AgentWake;
    }
    
    // 3. Check subscriptions (multiple) — deliver to all matching subscriptions
    let matches: Vec<_> = self.routing_table.subscriptions
        .iter()
        .filter(|(_, sub)| sub.category == event.category && sub.pattern_matches(&event.pattern))
        .map(|(id, sub)| (id.clone(), sub.subscriber.clone()))
        .collect();
    if !matches.is_empty() {
        return RoutingDecision::Subscribers(matches);
    }
    
    // 4. No handler, no subscription, not flagged → drop silently
    //    (local resolution is the default, which means "do nothing")
    RoutingDecision::Drop
}
```

### Routing Decision Types

| Decision | Description |
|----------|-------------|
| `Handler(ComponentId)` | Deliver to this specific handler |
| `Subscribers(Vec<(SubscriberId, ComponentId)>)` | Deliver to all matching subscribers |
| `AgentWake` | Wake the Agent Core |
| `Drop` | Drop the event (no action) |

---

## Scheduler

### Scheduled Event Structure

```rust
struct ScheduledEvent {
    /// Unique identifier
    id: Uuid,
    
    /// When to trigger this event
    trigger_time: DateTime,
    
    /// The event to emit when triggered
    event: Event,
    
    /// If true, re-schedule after trigger
    recurring: bool,
    
    /// For recurring events: how often to repeat
    interval: Option<Duration>,
    
    /// For recurring events: maximum number of repetitions (None = infinite)
    max_repetitions: Option<u64>,
    
    /// Number of times this event has been triggered
    repetition_count: u64,
    
    /// Who scheduled this event
    scheduled_by: ComponentId,
    
    /// When this event was scheduled
    scheduled_at: DateTime,
}
```

### Scheduler Implementation

The scheduler uses a **min-heap** (priority queue) to efficiently find the next event to trigger:

```rust
struct Scheduler {
    /// Min-heap of scheduled events, ordered by trigger_time
    heap: BinaryHeap<Reverse<ScheduledEvent>>,  // Reverse for min-heap
    
    /// Map from event ID to the event (for cancellation)
    events: HashMap<Uuid, ScheduledEvent>,
    
    /// Background thread that checks for due events
    worker: JoinHandle<()>,
}
```

The worker thread:
1. Sleeps until the next event's trigger time
2. Wakes up, checks the heap for all events with `trigger_time <= now()`
3. For each due event:
   - Constructs the event
   - Injects it into the routing pipeline
   - If recurring, re-schedules the next occurrence
4. Goes back to sleep

### Cron Grammar

The scheduler supports a simplified cron-like grammar:

```
cron = "@every" duration | "@daily" | "@hourly" | standard_cron
standard_cron = minute hour day month weekday
```

Where `duration` is a Go-style duration string:
- `"5s"` — 5 seconds
- `"10m"` — 10 minutes
- `"1h"` — 1 hour
- `"1d"` — 1 day

**Examples:**
- `@every 5s` — every 5 seconds
- `@hourly` — every hour at minute 0
- `@daily` — every day at 00:00
- `0 * * * *` — every hour at minute 0
- `*/5 * * * *` — every 5 minutes

---

## Coalescing

### Agent Core Wake Coalescing

Agent Core wakes are coalesced **per category**. If the agent is already processing a wake for category `health`, and a second `health` event arrives:

1. The bus does **not** send a second wake
2. Instead, it stores a flag: `health_events_dropped_since_last_wake = true`
3. When the agent's current wake finishes, the bus includes this flag in the next wake context

The dropped events themselves are **not** replayed; only the fact that "at least one more occurred" is delivered.

### Coalescing Configuration

```rust
struct CoalescingConfig {
    /// Maximum number of coalesced events per category
    max_coalesced_per_category: usize,
    
    /// Time window for coalescing (events within this window are coalesced)
    coalescing_window: Duration,
    
    /// Categories that should never be coalesced
    never_coalesce: Vec<Category>,
}
```

Default: `max_coalesced_per_category = 10`, `coalescing_window = 100ms`

---

## Inspection

### Query: Why Does an Event Wake the Agent?

The bus supports a query to explain routing decisions:

```rust
struct RoutingExplanation {
    event_category: Category,
    event_pattern: String,
    
    /// The final routing decision
    decision: RoutingDecision,
    
    /// Why this decision was made
    reason: RoutingReason,
}

enum RoutingReason {
    /// There is an explicit handler for this (category, pattern)
    ExplicitHandler { handler: ComponentId },
    
    /// The event is flagged as requires_decision
    RequiresDecision,
    
    /// There are matching subscribers
    MatchingSubscribers { count: usize },
    
    /// No handler, no subscribers, not flagged → dropped
    NoMatch,
}
```

**MCP Method:**
```json
// Request
{"method": "event.explain_routing", "params": {"category": "health", "pattern": "lambda.crash"}}

// Response
{"result": {
  "decision": "AgentWake",
  "reason": "RequiresDecision"
}}
```

### Query: What Events Wake the Agent?

List all event patterns that currently wake the Agent Core:

```json
// Request
{"method": "event.list_agent_wakes"}

// Response
{"result": {
  "patterns": [
    {"category": "input", "pattern": "text.new"},
    {"category": "health", "pattern": "lambda.crash"},
    {"category": "lambda", "pattern": "*.output"}
  ]
}}
```

---

## MCP Interface

### Methods

#### `event.emit(category: string, pattern: string, payload: Value, requires_decision: bool) → {}`

Emit an event to the bus.

**Parameters:**
- `category` — the event category
- `pattern` — the event pattern
- `payload` — the event payload (JSON-serializable)
- `requires_decision` — if true, this event will wake the Agent Core if no handler exists

**Errors:**
- `E_INVALID_CATEGORY` — category is not valid
- `E_INVALID_PATTERN` — pattern syntax is invalid

**Example:**
```json
// Request
{"method": "event.emit", "params": {
  "category": "health",
  "pattern": "lambda.crash",
  "payload": {"lambda": "video_player", "exit_code": 1},
  "requires_decision": true
}}

// Response
{}
```

#### `event.subscribe(category: string, pattern: string, since_revision: u64) → {subscription_id: string}`

Subscribe to events matching the category and pattern.

**Parameters:**
- `category` — the category to subscribe to
- `pattern` — the pattern to match
- `since_revision` — only deliver events with state revision > this value

**Returns:**
- `subscription_id` — unique identifier for this subscription (used for unsubscribing)

**Example:**
```json
// Request
{"method": "event.subscribe", "params": {
  "category": "lambda",
  "pattern": "video_player.*",
  "since_revision": 0
}}

// Response
{"result": {"subscription_id": "sub-1234"}}
```

#### `event.unsubscribe(subscription_id: string) → {}`

Unsubscribe from events.

**Parameters:**
- `subscription_id` — the subscription to remove

**Example:**
```json
// Request
{"method": "event.unsubscribe", "params": {"subscription_id": "sub-1234"}}

// Response
{}
```

#### `event.schedule(cron: string, payload: Value, recurring: bool) → {event_id: string}`

Schedule an event.

**Parameters:**
- `cron` — the schedule specification (cron grammar or @every)
- `payload` — the event payload
- `recurring` — if true, the event will repeat according to the cron

**Returns:**
- `event_id` — unique identifier for this scheduled event (used for cancellation)

**Errors:**
- `E_INVALID_CRON` — cron syntax is invalid
- `E_PERMISSION_DENIED` — caller lacks `CAP_TIMER` capability

**Example:**
```json
// Request
{"method": "event.schedule", "params": {
  "cron": "@every 5m",
  "payload": {"type": "health_check"},
  "recurring": true
}}

// Response
{"result": {"event_id": "evt-5678"}}
```

#### `event.cancel(event_id: string) → {}`

Cancel a scheduled event.

**Parameters:**
- `event_id` — the event to cancel

**Example:**
```json
// Request
{"method": "event.cancel", "params": {"event_id": "evt-5678"}}

// Response
{}
```

#### `event.stats() → Stats`

Get bus statistics.

**Returns:**
```json
{
  "events_emitted": 1234,
  "events_routed_to_handler": 1000,
  "events_routed_to_subscribers": 150,
  "events_routed_to_agent": 50,
  "events_dropped": 34,
  "agent_wakes": 25,
  "agent_wakes_coalesced": 10,
  "scheduled_events": 5,
  "subscriptions": 15,
  "uptime": 3600.5
}
```

---

## Integration with State Store

The Event Bus uses the State Store's `state.watch` mechanism for subscriptions. When a component subscribes to an event pattern:

1. The Event Bus creates a subscription in its internal table
2. If the pattern corresponds to a State Store path (e.g., `state:task.functions.*`), the Event Bus also creates a `state.watch` subscription
3. When the State Store emits a patch event, the Event Bus converts it to an Event Bus event and routes it

This ensures that state changes automatically generate events without requiring explicit `event.emit` calls.

---

## Performance

### Targets

| Metric | Target |
|--------|--------|
| Event routing latency (p99) | < 500μs |
| Event emission latency (p99) | < 100μs |
| Scheduler precision | < 10ms |
| Memory usage | < 50MB |

### Optimizations

1. **O(1) routing** — hash map lookup for (category, pattern)
2. **Batch delivery** — multiple events to the same subscriber are batched
3. **Lazy subscription** — subscriptions are only active when there are matching events
4. **Minimal copying** — events are reference-counted, not cloned

---

## See Also

- [State Store](./state-store.md) — for the underlying storage and subscriptions
- [Agent Core](./agent-core.md) — for the primary consumer of wake events
- [Lambda Server](./lambda-server.md) — for lambda health events
- [Policy Broker](./policy-broker.md) — for capability enforcement on scheduling
