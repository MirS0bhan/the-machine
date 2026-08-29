# Policy Broker

> **Overlap warning:** This directory contains **two implementations**.
> See [Python ↔ Rust Overlap Guide](../docs/guides/python-rust-overlap.md).

| | Path | Role |
|---|------|------|
| **Python (canonical for logic & tests)** | `policy_broker/` | Full rule interpreter, audit, rate limits |
| **Rust (boot placeholder)** | `src/` | Unix-socket daemon; deny-by-default until ported |

## Run (Python — for tests and policy development)

```bash
pip install -e .
uvicorn policy_broker.mcp_server:app --host 127.0.0.1 --port 8001
pytest tests/
```

## Run (Rust — boot/ISO path)

```bash
cargo run --bin policy-broker
# listens on /run/the-machine/policy-broker.sock
```

## Tests

Integration tests import `PolicyInterpreter` from Python directly — they do **not**
exercise either daemon over the network.
