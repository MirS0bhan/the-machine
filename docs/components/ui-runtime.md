# UI Runtime

**Layer:** L5  
**Type:** Deterministic, non-LLM  
**Language:** Rust (renderer) + Python/JavaScript (AUIL/ASL runtime)  
**Dependencies:** State Store, MCP Bus, Compositor  

---

## Overview

The UI Runtime is a **declarative renderer** that consumes the **UI State Tree** (from the State Store) and draws it — conceptually similar to a React renderer consuming a virtual DOM. The agent emits *patches* to the tree, not full re-renders, so existing state (scroll position, playback position, form input) survives.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  UI Runtime                                                       │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  State Store Client                                        │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ state.watch │  │ state.get   │  │ state.patch         │ │ │
│  │  │ (UI tree)   │  │ (bindings)  │  │ (reflecting changes)│ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Patch Engine                                              │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Parse       │  │ Validate    │  │ Apply               │ │ │
│  │  │ Patch Ops   │  │ (against    │  │ (incremental        │ │ │
│  │  │             │  │  current    │  │  updates)           │ │ │
│  │  │             │  │  tree)      │  │                     │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  ASL Engine                                                │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Mixin       │  │ Token       │  │ State-driven        │ │ │
│  │  │ Resolution  │  │ Resolution  │  │ Style               │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Renderer                                                   │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Node        │  │ Layout      │  │ Compositor          │ │ │
│  │  │ Traversal   │  │ Engine      │  │ Surface             │ │ │
│  │  │             │  │ (stack/grid)│  │ Management          │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  MCP Client (for event bindings)                           │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## AUIL (Agent UI Language)

### Node Structure

```rust
struct UiNode {
    /// Unique identifier for this node
    id: String,
    
    /// The kind of node (container, text, media_surface, etc.)
    kind: NodeKind,
    
    /// Node properties
    props: HashMap<String, Value>,
    
    /// Children (for container nodes)
    children: Vec<UiNode>,
    
    /// ASL style mixin name
    asl_style: Option<String>,
    
    /// Bindings (state:* or mcp:)
    bindings: Vec<Binding>,
}
```

### Node Kinds

| Kind | Description | Props |
|------|-------------|-------|
| `Container` | A box that can contain other nodes | `direction`, `alignment`, `padding`, `gap` |
| `Text` | A text label | `content`, `font`, `size`, `color`, `alignment` |
| `MediaSurface` | A media playback surface | `url`, `controls`, `loop`, `volume` |
| `InputField` | A text input field | `placeholder`, `value`, `on_change` binding |
| `List` | A scrollable list | `items`, `item_template`, `on_select` binding |
| `Button` | A clickable button | `label`, `on_press` binding, `disabled` |
| `Chart` | A data visualization | `data`, `type`, `axes` |
| `ExternalSurface` | A legacy Wayland/X11 window | `wayland_surface_id`, `x11_window_id` |

### Bindings

Bindings connect UI elements to the rest of the system:

```rust
struct Binding {
    /// The type of binding
    kind: BindingKind,
    
    /// The target path or method
    target: String,
}

enum BindingKind {
    /// State Store path (two-way binding)
    /// Syntax: state:path
    State { path: String },
    
    /// MCP method invocation (one-way binding)
    /// Syntax: mcp:method
    Mcp { method: String },
}
```

**Examples:**
```json
// Two-way binding: input field reads/writes to state
{"kind": "state", "target": "ui.root.search_input.value"}

// One-way binding: button invokes MCP method
{"kind": "mcp", "target": "lambda.video_player.play"}
```

---

## Patch Protocol

### Patch Operations

The UI Runtime supports five patch operations:

| Operation | Syntax | Description |
|-----------|--------|-------------|
| Update | `~id(props)` | Update properties of node `id` |
| Insert | `+anchor: node` | Insert `node` at `anchor` position |
| Remove | `-id` | Remove node `id` and its descendants |
| Replace | `!id: node` | Replace subtree at `id` with `node` |
| Move | `@id → other-id` | Move subtree from `id` to `other-id` |

### Patch Validation

Before applying a patch, the UI Runtime validates it:

1. **Reference validation:** All referenced node IDs exist (or are valid for insertion)
2. **Type validation:** Node kinds are valid
3. **Binding validation:** Bindings are syntactically valid
4. **Capability validation:** The caller has `CAP_STATE_WRITE` for the affected paths

### Patch Application

```rust
fn apply_patch(ops: Vec<PatchOp>, current_tree: &mut UiTree) -> Result<(), PatchError> {
    // 1. Validate all ops
    for op in &ops {
        validate_op(op, current_tree)?;
    }
    
    // 2. Apply ops in order
    for op in ops {
        match op {
            PatchOp::Update { id, props } => {
                let node = current_tree.get_mut(&id)?;
                node.props.extend(props);
                // Re-evaluate bindings
                update_bindings(node);
            }
            PatchOp::Insert { anchor, node } => {
                current_tree.insert(anchor, node);
                // Subscribe to state bindings
                subscribe_bindings(&node);
            }
            PatchOp::Remove { id } => {
                // Unsubscribe from state bindings
                unsubscribe_bindings(&id);
                current_tree.remove(id);
            }
            PatchOp::Replace { id, node } => {
                // Unsubscribe old, subscribe new
                unsubscribe_bindings(&id);
                current_tree.replace(id, node);
                subscribe_bindings(&node);
            }
            PatchOp::Move { from, to } => {
                current_tree.move_node(from, to);
                // Bindings are preserved
            }
        }
    }
    
    // 3. Re-render affected nodes
    render_affected_nodes(&ops, current_tree)?;
    
    // 4. Reflect changes to State Store
    state_store.patch(ops)?;
    
    Ok(())
}
```

### Example Patch

```json
{
  "ops": [
    {"op": "~", "id": "ui.root.controls.play", "props": {"label": "Pause", "disabled": false}},
    {"op": "+", "anchor": "ui.root.controls.right_of_play", "node": {"kind": "button", "id": "stop", "label": "Stop"}},
    {"op": "-", "id": "ui.root.controls.old_button"}
  ]
}
```

---

## ASL (Adaptive Style Language)

### Mixins

Mixins are reusable style definitions stored in the State Store under `prefs.asl.mixins.*`:

```json
{
  "name": "Card",
  "tokens": {
    "background": "$colors.background",
    "padding": "$spacing.medium",
    "border_radius": "$rounding.medium",
    "border_width": "1px",
    "border_color": "$colors.border"
  },
  "states": {
    "selected": {
      "background": "$colors.primary",
      "border_color": "$colors.primary"
    },
    "hover": {
      "background": "$colors.hover"
    }
  }
}
```

### Token Resolution

Tokens are resolved from the current theme:

```rust
fn resolve_tokens(mixin: &Mixin, theme: &Theme) -> ResolvedStyle {
    let mut resolved = HashMap::new();
    for (key, value) in &mixin.tokens {
        resolved.insert(key.clone(), resolve_token(value, theme));
    }
    // ... handle state transitions
    resolved
}

fn resolve_token(token: &str, theme: &Theme) -> Value {
    if let Some(parsed) = parse_token_reference(token) {
        // e.g., "$colors.primary" → theme.colors.primary
        theme.lookup(&parsed).unwrap_or(token.into())
    } else {
        token.into()
    }
}
```

### Theme

Themes are stored in the State Store under `prefs.theme.*`:

```json
{
  "name": "dark",
  "colors": {
    "background": "#1a1a1a",
    "foreground": "#ffffff",
    "primary": "#007aff",
    "secondary": "#5856d6",
    "border": "#383838",
    "hover": "#2a2a2a"
  },
  "spacing": {
    "small": "4px",
    "medium": "12px",
    "large": "24px"
  },
  "rounding": {
    "small": "4px",
    "medium": "8px",
    "large": "16px"
  },
  "typography": {
    "body": {"font": "Inter", "size": "14px", "weight": "400"},
    "title": {"font": "Inter", "size": "20px", "weight": "700"}
  }
}
```

---

## Rendering

### Render Loop

```rust
fn render_loop() {
    loop {
        // 1. Wait for a patch from the State Store
        let patches = state_watch.wait_for_patches();
        
        // 2. Apply the patches
        for patch in patches {
            apply_patch(patch.ops);
        }
        
        // 3. Render the updated tree
        render_tree();
        
        // 4. Present to compositor
        compositor_present();
    }
}
```

### Layout Engine

The layout engine supports two layout modes:

1. **Stack:** Nodes are arranged in a single direction (horizontal or vertical)
   - Properties: `direction`, `alignment`, `padding`, `gap`

2. **Grid:** Nodes are arranged in a grid
   - Properties: `columns`, `rows`, `gap`, `alignment`

### Compositor Integration

The UI Runtime communicates with the compositor using the Wayland protocol:

1. **Surface creation:** Each node creates a `wl_surface` (or reuses an existing one)
2. **Damage tracking:** The UI Runtime tracks which nodes have changed and only updates those surfaces
3. **Input routing:** Input events from the compositor are routed to the appropriate node
4. **External surface embedding:** For `ExternalSurface` nodes, the UI Runtime uses `wl_subsurface` or `XReparentWindow`

---

## MCP Interface

> **Boot path (today):** the Rust `ui-runtime` daemon. Canonical honesty audit: `docs/design-system/08-ui-framework/03-docs-code-honesty.md`.

### Methods (Rust boot)

| Method | Purpose |
|---|---|
| `ui.patch` | Apply patch ops; sync surfaces |
| `ui.get` | Get one node (omit `id` → root) |
| `ui.tree` | Serialize subtree snapshot |
| `ui.bind` | Attach mcp/state binding to a node |
| `ui.event` | Pointer/key/wheel → local feedback + bindings |
| `ui.status` | Revision, focus, parser/text_stack, a11y/i18n/ime flags |
| `ui.focus.get` / `ui.focus.set` / `ui.focus.next` | Tree focus + compositor focus sync |
| `ui.theme.get` / `ui.theme.set` | Theme token bag |
| `ui.auil.parse` / `ui.auil.load` | Parse AUIL source → patch ops / load into tree |
| `ui.a11y.tree` | AT-SPI-shaped accessibility tree export |
| `ui.atspi.status` | D-Bus bridge status (`org.themachine.A11y`) |
| `ui.i18n.status` / `ui.i18n.t` / `ui.i18n.load` | Locale catalogs + string lookup |
| `ui.components.list` | Registered component / widget catalog |

Historical names `ui.get_tree` / `ui.get_node` are **not** registered; use `ui.tree` / `ui.get`.

### AUIL kinds (boot)

Painted interaction kinds: `text`, `field`/`input`, `button`, `toggle`, `slider`, `list`, `dialog`, `icon` (geometric), `media` (ffmpeg first frame or play affordance), `chart`. Layout: `stack`/`container` + real `grid` (`cols`/`col_span`/`rtl`). The PascalCase NodeKind table above is a historical/target sketch — boot tags are lowercase AUIL primitives.

### Example — `ui.patch`

```json
{"method": "ui.patch", "params": {"ops": [
  {"op": "~", "id": "ui.root.title", "props": {"text": "Welcome"}}
]}}
```

### Example — `ui.tree` / `ui.get`

```json
{"method": "ui.tree", "params": {}}
{"method": "ui.get", "params": {"id": "ui.chat_send"}}
```

---

## Performance

### Targets

| Metric | Target |
|--------|--------|
| Patch apply latency (p99) | < 5ms |
| Layout engine latency (p99) | < 16ms (60 fps) |
| Render latency (p99) | < 16ms (60 fps) |
| Memory usage | < 100MB |

### Optimizations

1. **Incremental updates** — only update nodes that changed
2. **Damage tracking** — only re-render damaged areas
3. **Node pooling** — reuse nodes to reduce allocation
4. **Layout caching** — cache layout results when nodes haven't changed

---

## See Also

- [State Store](./state-store.md) — for UI tree storage and subscriptions
- [Compositor](./compositor.md) — for rendering and input routing
- [Agent Core](./agent-core.md) — for UI patch generation
- [MCP Bus](./mcp-bus.md) — for event bindings
- [UI framework maturity](../design-system/08-ui-framework/) — boot vs toolkit gap analysis
