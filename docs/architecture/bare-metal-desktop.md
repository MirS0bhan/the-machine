# Bare-Metal Desktop Readiness Plan

**Goal:** Boot The Machine on real hardware with a graphical session, agent stack, and optional disk install.

## Phases

| Phase | Scope | Status |
|-------|--------|--------|
| **A — Boot & install (G13)** | debootstrap rootfs, kernel in `/boot`, GRUB `LABEL=the-machine`, GPU firmware + Mesa packages | Done |
| **B — Display (G14 + G16)** | sysfs/DRM mode query + `display.set_mode`; compositor DRM fb fix | Done |
| **C — Network (G14)** | `net.list_interfaces` via sysfs; `ip link` for up/down | Done |
| **D — Wayland session (G17)** | `wl_compositor` / `wl_output` / `wl_seat` globals on `wayland-server` scaffold | Done |
| **E — Polish** | PipeWire audio (`pactl`), wpa_supplicant wifi (`wpa_cli`), udev hotplug rules | Done |

## Boot paths

1. **ISO / initramfs** (demo) — `make iso` + USB boot. Needs host kernel with KMS + evdev.
2. **Installed rootfs** — `make rootfs-release && sudo build/installer/install.sh /dev/sdX`.

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

On hardware: confirm `/dev/dri/card0`, evdev nodes, `display.get_modes` returns EDID modes, and `WAYLAND_DISPLAY` socket accepts `wl_compositor` binds.
