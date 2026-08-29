# Agent Core

**Layer:** L4  
**Type:** Hybrid LLM Router (Local + Cloud)  
**Language:** Rust (harness) + Python/C++ (model runners)  
**Dependencies:** Local Model, Cloud Model, MCP Bus, Policy Broker  

---

## Overview

The Agent Core is the **decision-making brain** of The Machine. It is a thin harness that loads skills and prompts at runtime, runs a session loop, holds two model clients, and speaks MCP. It does not contain task-specific branching logic — every piece of task intelligence lives in skills loaded from the State Store.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Agent Core                                                    │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Session Loop                                              │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Wait for    │  │ Gather      │  │ Execute             │ │ │
│  │  │ Wake        │  │ Context     │  │ Plan                │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Router                                                     │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Intent      │  │ Complexity  │  │ Routing Decision    │ │ │
│  │  │ Classifier  │  │ Estimator   │  │ (Local / Cloud)     │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Model Clients                                              │ │
│  │  ┌─────────────┐  ┌─────────────────────────────────────┐ │ │
│  │  │ Tier A      │  │ Tier B                              │ │ │
│  │  │ (Local)     │  │ (Cloud)                            │ │ │
│  │  │             │  │                                     │ │ │
│  │  │ - Always on │  │ - On-demand                         │ │ │
│  │  │ - Low       │  │ - High latency                      │ │ │
│  │  │   latency   │  │ - Frontier-scale                    │ │ │
│  │  │ - Privacy   │  │ - Only for novel/complex tasks     │ │ │
│  │  │   sensitive │  │                                     │ │ │
│  │  └─────────────┘  └─────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Skill Library                                              │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Intent      │  │ UI Patch    │  │ Lambda              │ │ │
│  │  │ Classifier  │  │ Generator   │  │ Generation          │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  MCP Client (speaks to MCP Bus)                            │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Session Loop

### Overview

The Agent Core runs an infinite session loop:

1. **Wait for wake** — block on `event_bus.next_wake()`
2. **Gather context** — collect recent intents, UI tree summary, active lambdas, task history
3. **Classify intent** — use local model to classify intent + estimate complexity
4. **Route** — decide local or cloud
5. **Plan** — generate a plan using the chosen model
6. **Execute** — execute the plan (MCP calls)
7. **Loop** — go back to step 1

### Wake Context

```rust
struct WakeContext {
    /// Why the agent was woken
    wake_reason: WakeReason,
    
    /// Recent intents (capped at 10)
    recent_intents: Vec<IntentHistory>,
    
    /// Summary of the current UI tree (not full tree, to save tokens)
    current_ui_tree: Option<UiTreeSummary>,
    
    /// Active lambdas (capped at 20)
    active_lambdas: Vec<FunctionSummary>,
    
    /// Recent task history (capped at 5)
    task_history: Vec<TaskSummary>,
    
    /// Specific state snapshots requested by the skill
    state_snapshots: HashMap<String, Value>,
    
    /// Privacy tag (if true, cloud is structurally excluded)
    privacy_tag: bool,
}
```

### Wake Reason

```rust
enum WakeReason {
    /// User input (text or voice)
    UserInput { text: String, source: InputSource },
    
    /// State change that requires decision
    StateChange { path: String, value: Value },
    
    /// Health event
    Health { component: ComponentId, status: HealthStatus },
    
    /// Timer
    Timer { event_id: String, payload: Value },
    
    /// External event
    External { source: String, payload: Value },
}
```

---

## Routing

### Routing Logic

```rust
fn route_context(context: WakeContext) -> RoutingDecision {
    // 1. If privacy_tag is true, cloud is structurally excluded
    if context.privacy_tag {
        return RoutingDecision::Local;
    }
    
    // 2. Use local model to classify intent + estimate complexity
    let classification = local_model.classify_intent(&context);
    
    // 3. If known task pattern and low ambiguity → local
    if classification.intent.is_known() && classification.complexity < Complexity::Medium {
        return RoutingDecision::Local;
    }
    
    // 4. If local_only_mode is enabled → local
    if self.local_only_mode {
        return RoutingDecision::Local;
    }
    
    // 5. Else → cloud
    RoutingDecision::Cloud
}
```

### Local-Only Mode

`local_only_mode` is a hard system toggle that disables cloud escalation entirely:

- **Setting:** `agent.local_only_mode(true)` MCP call
- **Enforcement:** Structural, not the model's judgment
- **Default:** `false` (cloud is allowed)

### Complexity Estimator

The local model produces a `complexity` score:

| Complexity | Meaning | Routing |
|------------|---------|---------|
| Low | Routine task, known pattern | Local |
| Medium | Requires some reasoning, known but non-trivial | Local (or Cloud if user prefers) |
| High | Novel task, requires planning | Cloud |

---

## Skills

### Skill Structure

Skills are stored in the State Store under `task.agent_skills.*`:

```rust
struct Skill {
    /// Unique name for this skill
    name: String,
    
    /// Version number
    version: u64,
    
    /// What this skill applies to
    applies_to: Vec<SkillTrigger>,
    
    /// System prompt (loaded into the model context)
    system_prompt: String,
    
    /// Few-shot examples (for the model)
    few_shot_examples: Vec<Example>,
    
    /// Output schema (for validation)
    output_schema: Schema,
    
    /// The function that executes the skill
    /// (actually just a description for the model; the model itself generates the plan)
    description: String,
}
```

### Skill Loading

On each wake, the Agent Core:

1. Determines the `wake_reason` category
2. Looks up skills with matching `applies_to` triggers
3. Loads the highest-version matching skill
4. Prepends the skill's system prompt to the model context

### Example: Intent Classification Skill

```json
{
  "name": "intent-classification",
  "version": 3,
  "applies_to": ["category:input"],
  "system_prompt": "You are an intent classifier for The Machine. Given a user input, classify the intent and estimate complexity.",
  "few_shot_examples": [
    {"input": "play some music", "output": {"intent": "media_play", "complexity": "low"}},
    {"input": "what's the weather", "output": {"intent": "weather_query", "complexity": "low"}},
    {"input": "build a video player", "output": {"intent": "lambda_register", "complexity": "high"}}
  ],
  "output_schema": {
    "type": "object",
    "properties": {
      "intent": {"type": "string"},
      "complexity": {"type": "string", "enum": ["low", "medium", "high"]},
      "requires_cloud": {"type": "boolean"}
    }
  },
  "description": "Classifies user input into intent categories"
}
```

---

## Model Clients

### Tier A — Local Model

**Characteristics:**
- Small (few billion parameters)
- Quantized (4-bit or 8-bit)
- Always resident, low latency (< 100ms)
- No network dependency
- Handles: intent classification, routine UI patches, simple tasks
- Privacy-sensitive inputs are routed here by structural gate

**Implementation:**
- Uses llama.cpp, ONNX Runtime, or similar
- Loaded at boot and kept in memory
- Exposes MCP interface: `local_model.generate(prompt)`

**MCP Interface:**
```json
// Request
{"method": "local_model.classify_intent", "params": {"context": {...}}}

// Response
{"result": {"intent": "media_play", "complexity": "low", "requires_cloud": false}}
```

### Tier B — Cloud Model

**Characteristics:**
- Large (frontier-scale)
- High latency (first token > 1s)
- Network dependency
- Handles: novel tasks, multi-step planning, lambda synthesis
- Only invoked when local model flags high complexity

**Implementation:**
- Uses OpenAI API, Anthropic API, or similar
- MCP interface: `cloud_model.plan(context)`

**MCP Interface:**
```json
// Request
{"method": "cloud_model.plan", "params": {"context": {...}}}

// Response
{"result": {
  "plan": [
    {"action": "lambda.register", "manifest": {...}},
    {"action": "state.patch", "ops": [...]}
  ]
}}
```

---

## Execution

### Plan Execution

The Agent Core executes the plan by making MCP calls:

```rust
fn execute_plan(plan: Plan) -> Result<(), ExecutionError> {
    for step in plan.steps {
        match step.action {
            Action::LambdaInvoke { name, version, payload } => {
                let result = mcp_client.call("lambda.invoke", params!(name, version, payload));
                // Store result in context for subsequent steps
            }
            Action::LambdaRegister { manifest, artifact } => {
                let result = mcp_client.call("lambda.register", params!(manifest, artifact));
                // If successful, update local cache
            }
            Action::StatePatch { ops } => {
                let result = mcp_client.call("state.patch", params!(ops));
            }
            Action::PolicyCheck { method, request } => {
                let result = mcp_client.call("policy.check", params!(method, request));
                // If result is ALLOW, proceed; if DENY or CONFIRM, handle
            }
            Action::EventSubscribe { category, pattern } => {
                let result = mcp_client.call("event.subscribe", params!(category, pattern));
            }
            Action::EventSchedule { cron, payload } => {
                let result = mcp_client.call("event.schedule", params!(cron, payload));
            }
        }
    }
    Ok(())
}
```

### Failure Handling

**Local model crash:**
- Event Bus receives `health` event with `status: "not-ready"`
- Fallback Shell activates
- Agent Core does not auto-restart; restart is gated by the Broker's protected-unit policy

**Cloud model timeout:**
- If cloud doesn't respond within `CLOUD_TIMEOUT` (default 30s):
  - Log the timeout
  - Retry up to 2 times with exponential backoff
  - If all retries fail, fall back to Tier A's best-effort plan with a "cloud unavailable" note to the user

**Agent Core crash mid-turn:**
- The last emitted MCP calls may have already taken effect
- On restart, the Agent Core reads the current State Store revision to determine what was done
- It does **not** automatically retry the failed turn; the user must re-issue the intent

---

## MCP Interface

### Methods

#### `agent.status() → Status`

Get the agent's status.

**Returns:**
```json
{
  "status": "running" | "idle" | "degraded" | "offline",
  "local_model": "loaded" | "loading" | "unavailable",
  "cloud_model": "available" | "unavailable" | "disabled",
  "local_only_mode": true | false,
  "uptime": 3600.5,
  "pending_wakes": 0
}
```

#### `agent.interrupt() → {}`

Interrupt the current wake processing.

**Behavior:**
- Cancels the currently running plan
- Does **not** roll back already-executed MCP calls
- Logs the interruption

#### `agent.local_only_mode(enabled: bool) → {}`

Set local-only mode.

**Parameters:**
- `enabled` — if true, cloud model is disabled

**Example:**
```json
// Request
{"method": "agent.local_only_mode", "params": {"enabled": true}}

// Response
{}
```

#### `agent.skills.list() → SkillSummary[]`

List all loaded skills.

**Returns:**
- Array of skill summaries

**Example:**
```json
// Request
{"method": "agent.skills.list"}

// Response
{"result": [
  {"name": "intent-classification", "version": 3, "applies_to": ["category:input"]}
]}
```

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `THE_MACHINE_LOCAL_MODEL_PATH` | `/usr/share/the-machine/models/` | Path to local model files |
| `THE_MACHINE_CLOUD_API_URL` | `https://api.openai.com` | Cloud model API URL |
| `THE_MACHINE_CLOUD_API_KEY` | (none) | Cloud model API key |
| `THE_MACHINE_CLOUD_TIMEOUT` | `30s` | Cloud model timeout |
| `THE_MACHINE_LOCAL_ONLY_MODE` | `false` | Enable local-only mode by default |

### Command-Line Arguments

```
agent-core [OPTIONS]

Options:
  --local-model-path <PATH>   Path to local model files
  --cloud-api-url <URL>       Cloud model API URL
  --cloud-api-key <KEY>       Cloud model API key
  --cloud-timeout <DUR>       Cloud model timeout
  --local-only-mode           Enable local-only mode
  --help                      Show this help
```

---

## Performance

### Targets

| Metric | Target |
|--------|--------|
| Wake to first MCP call (local) | < 200ms |
| Wake to first MCP call (cloud) | < 5s |
| Plan execution (per step) | < 100ms |
| Memory usage | < 2GB (including local model) |

---

## Security Considerations

1. **Scoped like a lambda** — Agent Core has no special access path; every action is an ordinary MCP call
2. **Privacy structural** — privacy_tag is produced at the earliest point and is structurally enforced
3. **No direct kernel access** — all kernel actions go through System Daemon + Policy Broker
4. **Prompt-injection containment** — ingested content is treated as data, not as instructions
5. **Retire early, retire often** — the agent makes itself unnecessary for intent families as fast as possible

---

## See Also

- [Local Model](../local-model.md) — for Tier A model details
- [MCP Bus](../mcp-bus.md) — for method registration and resolution
- [Policy Broker](../policy-broker.md) — for capability enforcement
- [State Store](../state-store.md) — for skill storage and context
- [Event Bus](../event-bus.md) — for wake events
