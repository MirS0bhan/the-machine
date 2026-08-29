# The Machine

[![Build & Release Artifacts](https://github.com/MirS0bhan/the-machine/actions/workflows/build.yml/badge.svg)](https://github.com/MirS0bhan/the-machine/actions/workflows/build.yml)

Agent-Native OS — where an AI agent sits between human intent and system mechanisms.

## Status

**Hybrid implementation (Python → Rust migration in progress)** — several components exist in both languages. See the [overlap guide](./docs/guides/python-rust-overlap.md).

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
- [Python ↔ Rust overlap guide](./docs/guides/python-rust-overlap.md) — **read this if confused about duplicate components**
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

## CI / Artifacts

Every push to `main` runs [`.github/workflows/build.yml`](.github/workflows/build.yml):

| Job | Artifact | Contents |
|-----|----------|----------|
| `build-rust-components` | `rust-<name>` × 10 | Release binary per Rust daemon |
| `build-rust-examples` | `rust-lambda-examples` | `fn-add`, `fn-bad` |
| `build-python-components` | `python-<pkg>` × 6 | Wheel per Python package |
| `build-iso` | `boot-initramfs`, `boot-iso` | Initramfs + bootable ISO |
| `package-release` | `the-machine-release` | **Combined tarball** with all of the above + `manifest.json` |

Download from the Actions run → **Artifacts** tab, or locally:

```bash
make ci-package   # build/artifacts/ with rust/, python/wheels/, iso/, manifest.json
```

## License

Proprietary — all rights reserved.
