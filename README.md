# The Machine

Agent-Native OS — where an AI agent sits between human intent and system mechanisms.

## Status

🚧 **Conceptual architecture, pre-implementation** — code is a work-in-progress.

## Documentation

- [Full documentation](./docs/index.md)
- [Architecture overview](./docs/architecture/overview.md)
- [Component specs](./docs/components/)
- [Development guide](./docs/guides/development.md)

## Building

```bash
# Build all Rust crates
make build

# Build release
make build-release
```

## Running

Start individual services (each in its own terminal):

```bash
cargo run --bin system-daemon
cargo run --bin state-store
cargo run --bin mcp-bus
```

## Testing

```bash
make test
```

## License

Proprietary — all rights reserved.
