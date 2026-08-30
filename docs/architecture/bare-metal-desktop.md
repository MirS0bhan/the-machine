# Bare-Metal Desktop Readiness Plan

**Goal:** Boot The Machine on real hardware with a graphical session, agent stack, and optional disk install.

## End-user readiness

| Milestone | Status |
|-----------|--------|
| Software phases A–E | **Complete** |
| CI / QEMU validation | **Complete** (`build/test-rootfs-validate.sh`, `build/test-installer-grub.sh`, `build/hardware-smoke.sh`) |
| Physical hardware smoke test | **Operator task** — see [Bare-metal guide](../guides/bare-metal.md) |

The project is ready for **developers and early adopters** who can run the hardware checklist. It is **not** a general-consumer desktop OS until validated on diverse GPU/Wi-Fi hardware.

## Phases

| Phase | Scope | Status |
|-------|--------|--------|
| **A — Boot & install (G13)** | debootstrap rootfs, kernel in `/boot`, GRUB + `/etc/fstab` `LABEL=the-machine`, GPU firmware + Mesa packages | Done |
| **B — Display (G14 + G16)** | sysfs/DRM mode query + `display.set_mode`; compositor DRM fb fix | Done |
| **C — Network (G14)** | `net.list_interfaces` via rtnetlink; `net.set_interface_state` via RTM_SETLINK (`ip` fallback) | Done |
| **D — Wayland session (G17)** | `wl_compositor` / `wl_output` / `wl_seat` / `wl_shm`; surface commit → pixel paint | Done |
| **E — Polish** | PipeWire audio (`pactl`), wpa_supplicant wifi (`wpa_cli`), udev hotplug rules | Done |

**Future (optional):** wlroots `xdg-shell` compositing for conventional Wayland clients — not required for the agent-native UI path.

## Boot paths

1. **ISO / initramfs** (demo) — `make iso` + USB boot. Needs host kernel with KMS + evdev.
2. **Installed rootfs** — `make rootfs-release && sudo build/installer/install.sh /dev/sdX`.

## G13 validation

```bash
bash build/rootfs-validate.sh build/rootfs
bash build/test-installer-grub.sh   # loopback install + GRUB check (CI)
```

## Shared code

| Module | Purpose |
|--------|---------|
| `common::drm_sysfs` | EDID mode parsing from `/sys/class/drm` |
| `common::secrets` | Secret dir layout + safe reads |
| `common::paths::drm_device_path` | `THE_MACHINE_DRM_DEVICE` helper |
| `build/rootfs-common.sh` | `ROOTFS_SERVICES` list shared by mkrootfs/mkinitramfs/validate |

## Runtime requirements (bare metal)

| Variable | Purpose |
|----------|---------|
| `THE_MACHINE_SOCKET_DIR` | `/run/the-machine` |
| `XDG_RUNTIME_DIR` | Wayland socket dir (`/run/the-machine`) |
| `THE_MACHINE_COMPOSITOR_BACKEND` | `auto` → DRM → framebuffer → memory; set `wayland` for wl globals |
| `THE_MACHINE_WL_DISPLAY_BIND` | `1` to bind wl_display in `auto` mode |
| `THE_MACHINE_DRM_DEVICE` | `/dev/dri/card0` |
| `STATE_STORE_BACKEND` | `sled` for persistence |

## Wi-Fi credentials

Store PSK at `/run/the-machine/secrets/<credential_ref>` (mode 0600) and pass `credential_ref` to `net.connect_wifi`.

## Verification

```bash
make build && make test-all && make verify-docs
make rootfs-release   # when debootstrap available
```

On hardware: confirm `/dev/dri/card0`, evdev nodes, `display.get_modes` returns EDID modes, and Wayland clients can attach SHM buffers.
