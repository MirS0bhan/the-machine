# The Machine

[![Build & Release Artifacts](https://github.com/MirS0bhan/the-machine/actions/workflows/build.yml/badge.svg)](https://github.com/MirS0bhan/the-machine/actions/workflows/build.yml)

Agent-Native OS — where an AI agent sits between human intent and system mechanisms.

## Status

**Production boot path (Rust)** — Phases 1–7 implemented. Python packages remain the reference for integration tests and agent prototyping. See the [overlap guide](./docs/guides/python-rust-overlap.md).

| Layer | Component | Status |
|-------|-----------|--------|
| L0 | System Daemon | Rust — evdev input, sysfs-backed power/net/audio/display |
| L1 | Lambda Server | Rust (seccomp sandbox, synthesis) + Python (tests) |
| L1 | State Store | Rust (sled + watch) + Python (RocksDB/memory) |
| L1 | Event Bus | Rust (scheduler, D-Bus/inotify/audio adapters) |
| L2 | Policy Broker | Rust (rule engine, audit, confirmation UI) |
| L3 | MCP Bus | Rust (registry, policy middleware, leases) |
| L4 | Agent Core | Rust (LLM planner, cloud router, skills) |
| L4 | Local Model | Rust `local-model-daemon` (GGUF in initramfs) + Python reference |
| L4 | Marketplace | Rust (bundle install → lambda + ui.patch) |
| L5 | UI Engine | Python (AUIL/ASL parser, renderer) |
| L5 | UI Runtime | Rust (patch tree, binding execution) |
| L5 | Compositor | Rust (framebuffer `/dev/fb0`, confirmation surface) |
| L5 | Fallback Shell | Rust (console recovery) |

Canonical inventory: [`docs/reference/component-inventory.yaml`](./docs/reference/component-inventory.yaml) (verified by `make verify-docs`).

## Documentation

- [Full documentation](./docs/index.md)
- [Python ↔ Rust overlap guide](./docs/guides/python-rust-overlap.md)
- [Architecture overview](./docs/architecture/overview.md)
- [Runtime model](./docs/architecture/runtime-model.md)
- [Gap analysis](./docs/architecture/gap-analysis.md)
- [Component specs](./docs/components/)
- [Getting Started](./docs/guides/getting-started.md)
- [Development Guide](./docs/guides/development.md)
- [Testing & Coverage](./docs/guides/testing.md)

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

## Testing & verification

```bash
make test-all       # Rust + Python tests
make verify         # Full verification (tests, builds, docs, inventory)
make verify-docs    # Documentation ↔ code cross-check
make coverage       # Rust test coverage report (llvm-cov)
```

## CI / Artifacts

Every push to `main` runs [`.github/workflows/build.yml`](.github/workflows/build.yml):

| Job | Artifact | Contents |
|-----|----------|----------|
| `test` | — | Rust + Python tests, doc verification |
| `coverage` | `coverage-lcov` | LCOV report |
| `build-rust-components` | `rust-<name>` × 12 | Release binary per Rust daemon |
| `build-rust-examples` | `rust-lambda-examples` | `fn-add`, `fn-bad` |
| `build-python-components` | `python-<pkg>` × 6 | Wheel per Python package |
| `build-iso` | `boot-initramfs`, `boot-iso` | Initramfs + bootable ISO |
| `package-release` | `the-machine-release` | Combined tarball + `manifest.json` |

Download from the Actions run → **Artifacts** tab, or locally:

```bash
make ci-package   # build/artifacts/ with rust/, python/wheels/, iso/, manifest.json
```

## License

Proprietary — all rights reserved.
