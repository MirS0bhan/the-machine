# AUIL/ASL UI Engine

A declarative UI language and runtime for the Agent-Native OS.

[![Python 3.10+](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

The AUIL/ASL UI Engine provides a complete UI rendering pipeline for the Agent-Native OS:

- **AUIL (Agent UI Layout)**: Line-oriented, indentation-based structure language
- **ASL (Agent Style Language)**: Style and motion language with tokens and mixins
- **Patch Protocol**: Agent-to-UI update mechanism (patch-native, not render-native)
- **MCP Interface**: Agent interaction via Model Context Protocol
- **Renderer Abstraction**: Interface for Wayland compositor integration

## Design Principles

1. **Cheap for an LLM to emit and patch** - Terse grammar, token-efficient
2. **Patch-native, not render-native** - Agent emits diffs, not full trees
3. **Structure and style are separate languages** - AUIL for structure, ASL for style
4. **Real-time stays out of the agent's path** - Hover/press resolved locally
5. **Composition over regeneration** - Components with mixins and inheritance

## Quick Start

### Using the Engine

```python
from ui_engine import create_engine

# Create engine
engine = create_engine()

# Render a UI
engine.render('''
stack#root dir=v gap=m
  text(role=title) "Hello World"
  button#ok label=OK on:press=mcp:app.confirm
''')

# Apply patches
engine.patch("~ok(color=accent)")

# Handle events
engine.handle_mcp_tool("ui.event", {
    "event_type": "hover",
    "node_id": "ok",
})
```

### Using the SDK Directly

```python
from auil_parser import parse_auil
from asl_parser import parse_asl
from runtime import UIRuntime

# Parse AUIL
tree = parse_auil('''
stack#root dir=v
  text(role=title) "Hello World"
  button#ok label=OK
''')

# Parse ASL
styles = parse_asl('''
token surface.primary = adaptive(light:#FFFFFFEE dark:#1E1E1EEE)
style Surface
  bg=token:surface.primary
  radius=8
''')

# Create runtime
runtime = UIRuntime()
runtime.load_auil('''
stack#root dir=v
  text "Hello World"
''')
```

## AUIL Syntax

AUIL is line-oriented and indentation-based:

```
tag#id.mixin1.mixin2 prop1=val1 prop2=val2 "text content"
  child1
  child2
```

### Example

```
stack#root dir=v gap=m
  text(role=title) "Video Player"
  media#video type=video src=$lambda:video_player.stream
  stack#controls dir=h gap=s align=center
    button#play label=Play on:press=mcp:video_player.play
    button#pause label=Pause on:press=mcp:video_player.pause
    slider#progress max=100 value=@player.position
```

## ASL Syntax

ASL defines tokens, scales, motions, and style mixins:

```
token surface.primary = adaptive(light:#FFFFFFEE dark:#1E1E1EEE)
token accent = #007AFF

scale radius: sm=6 md=10 lg=16
scale space: xs=4 sm=8 md=12 lg=16 xl=24

motion snappy = spring(stiffness=300 damping=26)
motion gentle = duration(300ms ease=ease-out)

style Surface
  bg=token:surface.primary
  radius=r-lg

style Hoverable
  on:hover => scale=1.02 motion=snappy
  on:press => scale=0.98 motion=snappy
```

## Patch Protocol

The agent emits patches, not full trees:

```
~footer(color=accent style=compact)
+footer/append: text(role=caption) "Copyright"
-old-banner
@move-me → destination/0
```

### Operators

| Op | Purpose |
|---|---|
| `~` | Update properties in place |
| `+` | Insert node |
| `-` | Remove node |
| `!` | Replace subtree |
| `@` | Move/reorder node |

## MCP Tools

| Tool | Purpose |
|---|---|
| `ui.render` | Render a UI tree or apply patches |
| `ui.patch` | Apply patches to existing UI |
| `ui.state` | Get current UI state |
| `ui.event` | Handle UI events |
| `ui.register_style` | Register ASL styles |
| `ui.register_component` | Register components |
| `ui.resolve_intent` | Resolve MCP intents |
| `ui.get_tree` | Get full UI state tree |
| `ui.get_stats` | Get engine statistics |

## Components

Built-in components with inheritance:

- **Surface** - Base surface with styling
- **Card** - Surface + Hoverable
- **ListRow** - Surface + Hoverable + Pressable
- **PrimaryButton** - Surface + Hoverable + Pressable
- **IconBtn** - Surface + Hoverable + Pressable
- **Field** - Surface with label and input slots
- **MediaPlayer** - Surface with video and controls slots
- **Chart** - Surface with data slot

### Custom Components

```python
engine.handle_mcp_tool("ui.register_component", {
    "name": "VideoCard",
    "parent": "Card",
    "mixins": ["VideoPlayer"],
})
```

## Real-Time Events

Events are handled locally without MCP round-trip:

- **hover** - Applied locally (visual feedback)
- **press/release** - Applied locally (visual feedback)
- **focus/blur** - Applied locally (accessibility)
- **drag** - Applied locally (interaction)

Semantic actions route to agent:

- **click with `on:press=mcp:...`** - Routes to agent via MCP

## Project Structure

```
ui-engine/
├── docs/
│   └── spec.md              # AUIL + ASL specification
├── models.py                # Core data models
├── auil_parser.py           # AUIL parser
├── asl_parser.py            # ASL parser
├── patch_protocol.py        # Patch protocol
├── components.py            # Component registry
├── runtime.py               # UI runtime
├── renderer.py              # Renderer abstraction
├── mcp_interface.py         # MCP control interface
├── server.py                # Main engine
├── test_engine.py           # Test suite
├── pyproject.toml           # Project configuration
└── README.md                # This file
```

## Wayland Compositor Integration

The UI Engine provides an abstract renderer interface for Wayland compositor integration:

```python
from renderer import AbstractRenderer

class WaylandRenderer(AbstractRenderer):
    def create_surface(self, surface_id, properties):
        # Create Wayland surface
        pass
    
    def update_surface(self, surface_id, properties):
        # Update surface properties
        pass
    
    def destroy_surface(self, surface_id):
        # Destroy surface
        pass
```

The Wayland compositor is a separate project that implements this interface.

## Development

### Running Tests

```bash
# Run all tests
python test_engine.py

# Or with pytest
pytest
```

### Type Checking

```bash
mypy ui_engine/
```

### Linting

```bash
ruff check ui_engine/
```

## Architecture

```
┌───────────────────────────────────────────────────────────────┐
│  Agent Core                                                    │
│                                                                │
│   ┌───────────────┐  ┌────────────────┐  ┌──────────────────┐ │
│   │ AUIL Parser    │  │ ASL Parser      │  │ Patch Protocol   │ │
│   │ (structure)    │  │ (style/motion)  │  │ (updates)        │ │
│   └───────┬────────┘  └────────┬────────┘  └────────┬────────┘ │
│           │                    │                     │          │
│   ┌───────▼────────────────────▼─────────────────────▼───────┐│
│   │                    UI Runtime                             ││
│   │   - UI State Tree (addressed by stable IDs)               ││
│   │   - Real-time state (hover, press) resolved locally       ││
│   │   - Semantic actions route to agent via MCP               ││
│   └───────────────────────────┬──────────────────────────────┘│
│                                │                               │
│   ┌───────────────────────────▼──────────────────────────────┐│
│   │              Renderer Abstraction                         ││
│   │   - Abstract interface for Wayland compositor            ││
│   │   - Create/update/destroy surfaces                        ││
│   │   - Batch commit for performance                          ││
│   └──────────────────────────────────────────────────────────┘│
└───────────────────────────────────────────────────────────────┘
```

## Open Items (from spec)

1. **ASL `state:` prefix** (for explicit state transitions)
2. **Component slot defaults** (content when slot unfilled)
3. **Token inheritance** (e.g., `surface.frosted = surface.primary + vibrancy`)
4. **Style attachments** (adding mixins dynamically)
5. **Layout algorithm** (flex-like for stack, grid template for grid)
6. **Accessibility mapping** (semantic roles → AT output)
7. **Compositor-level theming** (system dark/light switch broadcast)

## License

MIT
