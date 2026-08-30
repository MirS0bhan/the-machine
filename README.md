# The Machine

[![Build & Release Artifacts](https://github.com/MirS0bhan/the-machine/actions/workflows/build.yml/badge.svg)](https://github.com/MirS0bhan/the-machine/actions/workflows/build.yml)

Agent-native operating system — an AI agent sits between human intent and system mechanisms.

## Readiness

| Track | Status |
|-------|--------|
| **Developer / QEMU / CI** | Production boot path — `make iso`, `make test-all`, `make verify` |
| **Bare-metal desktop (early adopters)** | Software complete — ISO + rootfs installer, DRM/Wayland, rtnetlink, wifi, audio |
| **General end users** | **Not yet** — needs your hardware smoke test ([checklist](./docs/guides/bare-metal.md#hardware-smoke-checklist)) |

The bare-metal stack (phases A–E) is implemented and validated in CI. Shipping to arbitrary end-user hardware still requires confirming GPU, Wi-Fi, and disk on a physical machine.

## What's included

| Layer | Component | Bare-metal notes |
|-------|-----------|------------------|
| L0 | System Daemon | evdev input, DRM display, rtnetlink, `wpa_cli`, `pactl` |
| L1–L4 | Agent stack | MCP bus, policy-broker, state-store, agent-core, local-model-daemon, marketplace |
| L5 | Compositor | DRM/framebuffer pixel output; Wayland `wl_shm` surface paint |
| Build | G13 installer | debootstrap rootfs, GRUB, `build/rootfs-validate.sh` |

Full inventory: [`docs/reference/component-inventory.yaml`](./docs/reference/component-inventory.yaml) (verified by `make verify-docs`).

## Documentation

| Doc | Description |
|-----|-------------|
| [Documentation index](./docs/index.md) | Full doc map |
| [Bare-metal guide](./docs/guides/bare-metal.md) | Install, runtime env, hardware checklist |
| [Getting started](./docs/guides/getting-started.md) | Dev host build & run |
| [Architecture](./docs/architecture/overview.md) | System design |
| [Gap analysis](./docs/architecture/gap-analysis.md) | Living checklist |
| [Component specs](./docs/components/) | Per-service reference |

## Quick start (development)

```bash
make build
make test-all
make verify-docs
make services-start    # local harness
make iso               # bootable image
```

## Bare-metal install

```bash
make build-release
make rootfs-release                              # debootstrap + sudo
bash build/rootfs-validate.sh build/rootfs     # pre-flight check
sudo build/installer/install.sh /dev/sdX         # destructive — picks target disk
```

See [Bare-metal guide](./docs/guides/bare-metal.md) for Wi-Fi credentials, display modes, and the hardware smoke checklist.

## QEMU

```bash
make qemu          # kernel + initramfs (fast)
make run           # ISO with graphics when $DISPLAY is set
```

## CI artifacts

Every push to `main` runs [`.github/workflows/build.yml`](.github/workflows/build.yml). Download ISO, initramfs, and per-service binaries from the Actions **Artifacts** tab, or locally:

```bash
make ci-package
```

## Verification

```bash
make test-all           # Rust + Python + build script tests
make verify             # Full verification suite
make verify-docs        # Docs ↔ code cross-check
bash build/hardware-smoke.sh   # bare-metal software smoke
```

## License

Proprietary — all rights reserved.
