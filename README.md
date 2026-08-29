# The Machine

Agent-Native OS — where an AI agent sits between human intent and system mechanisms.

## Status

**Hybrid implementation** — Rust boot daemons + Python MCP reference servers, with integration tests and an ISO build pipeline.

| Layer | Component | Status |
|-------|-----------|--------|
| L0 | System Daemon | Rust daemon (mock kernel ops) |
| L1 | Lambda Server | Python (full) + Rust (sandbox) |
| L1 | State Store | Python (memory/RocksDB) + Rust (in-mem) |
| L1 | Event Bus | Rust (full) + Python (integration tests) |
| L2 | Policy Broker | Python (full) + Rust (stub) |
| L3 | MCP Bus | Rust (routing to component sockets) |
| L4 | Agent Core | Rust (session loop + heuristic router) |
| L4 | Local Model | Python (llama-cpp wrapper) |
| L5 | UI Engine | Python (AUIL/ASL parser, renderer) |
| L5 | UI Runtime | Rust (in-memory tree) |
| L5 | Compositor | Rust (logical model) |
| L5 | Fallback Shell | Rust (console recovery) |

## Documentation

- [Full documentation](./docs/index.md)
- [Architecture overview](./docs/architecture/overview.md)
- [Component specs](./docs/components/)
- [Getting Started](./docs/guides/getting-started.md)
- [Development Guide](./docs/guides/development.md)

## Building

```bash
# Build all Rust crates
make build

# Build release binaries + bootable ISO
make build-release
make iso
```

## Running

Start all services (development harness):

```bash
make services-start
make services-stop
```

Boot in QEMU:

```bash
make qemu    # kernel + initramfs
make run     # ISO
```

## Testing

```bash
make test          # all tests
make test-rust     # cargo test --workspace
make test-python   # pytest integration + component tests
```

## License

Proprietary — all rights reserved.
