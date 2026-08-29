# Getting Started

This guide walks through building, testing, and running **The Machine** on a Linux development host.

## Prerequisites

> **Python ↔ Rust overlap:** Several components exist in both languages during migration.
> Read [Python ↔ Rust Overlap Guide](./python-rust-overlap.md) before picking which to run.

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.98+ (see `rust-toolchain.toml`) | L0–L5 service daemons |
| Python | 3.10+ | L1 MCP servers (lambda, policy, state, local-model, ui-engine) |
| busybox-static | any | Initramfs `/bin/sh` |
| grub-pc-bin, xorriso | any | ISO image creation |
| pytest, pytest-asyncio | any | Integration tests |

Install system packages on Debian/Ubuntu:

```bash
sudo apt-get install -y busybox-static grub-pc-bin xorriso
```

Install Python packages:

```bash
pip install -e lambda-server -e policy-broker -e state-store -e local-model \
            -e ui-engine -e event-bus
pip install pytest pytest-asyncio markdown
```

## Build

```bash
# All Rust crates
make build

# Release binaries (for ISO)
make build-release

# Bootable initramfs + ISO
make iso
```

## Run (development)

```bash
# Rust daemons (matches ISO boot)
make services-start

# Rust bus + Python policy/lambda (full rule engine, separate storage)
THE_MACHINE_RUNTIME=hybrid ./scripts/start-services.sh

# Python HTTP servers only
THE_MACHINE_RUNTIME=python ./scripts/start-services.sh

make services-stop
```

Or start individual Rust daemons:

```bash
export THE_MACHINE_SOCKET_DIR=/tmp/the-machine/run
mkdir -p $THE_MACHINE_SOCKET_DIR
cargo run --bin mcp-bus &
cargo run --bin event-bus &
cargo run --bin agent-core &
```

## Test

```bash
make test          # Rust workspace tests
make test-python   # Python unit + integration tests
make test-all      # Everything
```

## Boot in QEMU

```bash
make qemu          # kernel + initramfs (fast iteration)
make run           # boot the ISO
```

Set `KERNEL=/path/to/vmlinuz` if auto-detection fails.

## Documentation

```bash
make docs          # build docs/build/index.html
make -C docs serve # http://localhost:8000
```
