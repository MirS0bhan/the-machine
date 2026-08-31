# Getting Started

This guide walks through building, testing, and running **The Machine** on a Linux development host.

## Prerequisites

> **Python ↔ Rust overlap:** Several components exist in both languages during migration.
> Read [Python ↔ Rust Overlap Guide](./python-rust-overlap.md) before picking which to run.

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.98+ (see `rust-toolchain.toml`) | L0–L5 service daemons |
| Python | 3.10+ | L1 MCP servers (lambda, policy, state, local-model, ui-engine) |
| busybox-static | any | Initramfs `/bin/sh` (or auto-fetched via `build/fetch-busybox.sh`) |
| grub-pc-bin, xorriso, mtools | any | ISO image creation (`grub-mkrescue` needs `mformat` from mtools) |
| pytest, pytest-asyncio | any | Integration tests |
| cargo-llvm-cov | optional | `make coverage` |

Install system packages on Debian/Ubuntu:

```bash
sudo apt-get install -y busybox-static grub-pc-bin grub-common xorriso mtools \
    libfreetype6-dev pkg-config
```

Install Python packages:

```bash
pip install -e lambda-server -e policy-broker -e state-store -e local-model \
            -e ui-engine -e event-bus
pip install pytest pytest-asyncio markdown pyyaml
```

## Build

```bash
# All Rust crates (12 services + common + examples)
make build

# Release binaries (for ISO)
make build-release

# Bootable initramfs + ISO (fetches GGUF model + busybox when missing)
make iso
```

## Run (development)

```bash
# Rust daemons (matches ISO boot — default)
make services-start

# Rust bus + Python policy/lambda (full rule engine, separate storage)
THE_MACHINE_RUNTIME=hybrid ./scripts/start-services.sh

# Python HTTP servers only
THE_MACHINE_RUNTIME=python ./scripts/start-services.sh

make services-stop
```

Boot services started by `scripts/start-services.sh` (rust mode):

`system-daemon` → `policy-broker` → `mcp-bus` → `state-store` → `event-bus` → `lambda-server` → `local-model-daemon` → `marketplace` → `agent-core` → `ui-runtime` → `compositor`

### Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `THE_MACHINE_SOCKET_DIR` | `/tmp/the-machine/run` | Unix socket directory |
| `THE_MACHINE_RUNTIME` | `rust` | `rust` \| `hybrid` \| `python` |
| `STATE_STORE_BACKEND` | `memory` (dev), `sled` (boot) | Persistence backend |
| `STATE_STORE_PATH` | — | sled database path |
| `LOCAL_MODEL_PATH` | `/models/machine-tiny.gguf` (boot) | GGUF model file |
| `LOCAL_MODEL_HTTP_URL` | — | Proxy to Python llama.cpp server |
| `OPENAI_API_KEY` | — | Cloud LLM fallback for agent-core |
| `WAYLAND_DISPLAY` | `wayland-0` (boot) | Compositor display |
| `RUST_LOG` | `info` | Tracing filter |
| `THE_MACHINE_LAMBDA_DIR` | `/var/the-machine/lambdas` | Synthesized lambda source root |
| `THE_MACHINE_TOKEN_SECRET` | ISO default material | HMAC key for grant tokens |
| `THE_MACHINE_POLICY_FAIL_OPEN` | unset (fail closed) | Set `1` to allow mutations when the broker is down |
| `THE_MACHINE_LEASE_FAST_PATH` | unset (metadata only) | Set `1` to bind per-lease relay sockets on `bus.lease` |

## Test

```bash
make test-all      # Rust workspace + Python suites
make lint          # rustfmt + clippy (mcp-bus)
make verify        # Full verification: tests + builds + docs + inventory
make verify-docs   # Cross-check docs against component-inventory.yaml
make coverage      # Rust coverage via cargo llvm-cov
```

## Boot in QEMU

```bash
make qemu          # kernel + initramfs (fast iteration)
make run           # boot the ISO (graphical if $DISPLAY is set)
make run-console   # boot the ISO on the serial console (always nographic)
```

QEMU uses KVM when `/dev/kvm` is readable, otherwise TCG. Set
`KERNEL=/path/to/vmlinuz` if auto-detection fails.

## Documentation

```bash
make docs          # build docs/build/index.html
make -C docs serve # http://localhost:8000
```

Canonical component list: [`docs/reference/component-inventory.yaml`](../reference/component-inventory.yaml).
