# Event Bus

> **Overlap warning:** This directory contains **two implementations**.
> See [Python ↔ Rust Overlap Guide](../docs/guides/python-rust-overlap.md).

| | Path | Role |
|---|------|------|
| **Rust (canonical for production)** | `src/` | Full scheduler, routing, agent-wake coalescing |
| **Python (test harness only)** | `event_bus/` | In-process `EventRouter` for `tests/integration/` |

The Python package is **not** a second production server. It exists so integration tests
can call `event_bus.router.EventRouter` without spawning the Rust daemon.

## Run (Rust)

```bash
cargo run --bin event-bus
```

## Python (tests)

```python
from event_bus.router import EventRouter
router = EventRouter()
router.publish("task-complete", {"task_id": "t-001"})
```
