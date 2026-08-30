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

## Event adapters (Rust)

The production daemon spawns background adapters that publish into the local
`event-bus` socket:

| Adapter | Source | Events |
|---------|--------|--------|
| D-Bus (zbus) | system bus | `desktop.notify`, `login.prepare_sleep` |
| inotify | filesystem watches | `fs.change.*` |
| audio | PipeWire socket presence | `pipewire.state` |

Set `THE_MACHINE_DISABLE_DBUS=1` to skip the D-Bus adapter (e.g. initramfs
environments without a system bus). The adapter uses native **zbus** signal
streams and does not shell out to `dbus-monitor`.
