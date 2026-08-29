# State Store

> **Overlap warning:** This directory contains **two implementations**.
> See [Python ↔ Rust Overlap Guide](../docs/guides/python-rust-overlap.md).

| | Path | Role |
|---|------|------|
| **Python (canonical for persistence)** | `state_store/` | RocksDB or memory backend, FastAPI MCP |
| **Rust (boot daemon)** | `src/` | In-memory HashMap; Unix socket for ISO |

## Run (Python)

```bash
pip install -e .
STATE_STORE_BACKEND=memory uvicorn state_store.mcp_server:app --port 8002
pytest tests/
```

## Run (Rust)

```bash
cargo run --bin state-store
```

Data is **not shared** between Python and Rust instances.
