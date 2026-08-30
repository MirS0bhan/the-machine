# Bare-Metal Desktop Readiness Plan

**Goal:** Boot The Machine on real hardware with a graphical session, agent stack, and optional disk install.

## Phases

| Phase | Scope | Status |
|-------|--------|--------|
| **A — Boot & install (G13)** | debootstrap rootfs, kernel in `/boot`, GRUB `LABEL=the-machine`, GPU firmware + Mesa packages | In progress |
| **B — Display (G14 + G16)** | sysfs/DRM mode query + `display.set_mode`; compositor DRM fb fix | In progress |
| **C — Network (G14)** | `net.list_interfaces` via sysfs; `ip link` for up/down | In progress |
| **D — Wayland session (G17)** | wlroots seat/output on top of `wl_session.rs` scaffold | Open |
| **E — Polish** | PipeWire audio, wpa_supplicant wifi, hotplug udev | Open |

## Boot paths

1. **ISO / initramfs** (demo) — `make iso` + USB boot. Needs host kernel with KMS + evdev.
2. **Installed rootfs** — `make rootfs-release && sudo build/installer/install.sh /dev/sdX`.

## Runtime requirements (bare metal)

| Variable | Purpose |
|----------|---------|
| `THE_MACHINE_SOCKET_DIR` | `/run/the-machine` |
| `XDG_RUNTIME_DIR` | Wayland socket dir (`/run/the-machine`) |
| `THE_MACHINE_COMPOSITOR_BACKEND` | `auto` → DRM → framebuffer → memory |
| `THE_MACHINE_DRM_DEVICE` | `/dev/dri/card0` |
| `STATE_STORE_BACKEND` | `sled` for persistence |

## Verification

```bash
make build && make test-all && make verify-docs
make rootfs-release   # when debootstrap available
```

On hardware: confirm `/dev/dri/card0`, evdev nodes, and `display.get_modes` returns EDID modes.
