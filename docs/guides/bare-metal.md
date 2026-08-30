# Bare-Metal Desktop Guide

Install and run **The Machine** on physical hardware with a graphical desktop session.

## Readiness level

| Audience | Status |
|----------|--------|
| **Developers / early adopters** | Ready — ISO, rootfs installer, DRM/Wayland stack, network/audio/wifi adapters |
| **General end users** | Not yet — requires a physical hardware smoke test on your machine (see checklist below) |

Software phases A–E are implemented. What remains is validating **your** GPU, Wi-Fi chipset, and disk layout on real hardware.

---

## Quick start (USB demo)

```bash
make build-release
make iso
# Write build/the-machine.iso to USB, boot with UEFI/legacy BIOS
```

Boot requirements: x86_64 CPU, 1 GB+ RAM, kernel with KMS + evdev (most modern Linux live environments qualify for building the ISO).

---

## Full install (disk)

```bash
make build-release
make rootfs-release          # needs debootstrap + sudo on build host
sudo build/installer/install.sh /dev/sdX
```

The installer:
- Creates a GPT partition labeled `the-machine`
- Copies the rootfs and installs GRUB
- Expects `/vmlinuz` or `/boot/vmlinuz` in the rootfs

Validate a rootfs tree before writing to disk:

```bash
bash build/rootfs-validate.sh build/rootfs
bash build/test-installer-grub.sh   # loopback install + GRUB validation (CI)
```

For automated installs (e.g. CI), set `THE_MACHINE_INSTALLER_YES=1` to skip the confirmation prompt.

---

## Runtime environment

| Variable | Default (installed) | Purpose |
|----------|---------------------|---------|
| `THE_MACHINE_SOCKET_DIR` | `/run/the-machine` | MCP Unix sockets |
| `XDG_RUNTIME_DIR` | `/run/the-machine` | Wayland socket directory |
| `THE_MACHINE_COMPOSITOR_BACKEND` | `auto` | `auto` → DRM → framebuffer → memory |
| `THE_MACHINE_WL_DISPLAY_BIND` | unset | Set `1` to bind `wl_display` in `auto` mode |
| `THE_MACHINE_DRM_DEVICE` | `/dev/dri/card0` | DRM/KMS device |
| `STATE_STORE_BACKEND` | `sled` | Persistent state |
| `STATE_STORE_PATH` | `/var/lib/the-machine/state` | sled database path |

---

## Display

- **Agent API:** `display.get_modes`, `display.set_mode` (grant token required for set)
- **Compositor:** DRM/KMS dumb buffer with EDID mode discovery via `/sys/class/drm`
- **Wayland:** Set `THE_MACHINE_COMPOSITOR_BACKEND=wayland` or `THE_MACHINE_WL_DISPLAY_BIND=1` to expose `wl_compositor`, `wl_output`, `wl_seat`, and `wl_shm` with surface commit → pixel paint

---

## Network

- `net.list_interfaces` — rtnetlink (`RTM_GETLINK`), sysfs fallback
- `net.set_interface_state` — rtnetlink (`RTM_SETLINK`), `ip link` fallback
- `net.connect_wifi` — `wpa_cli` + credential file

Store Wi-Fi PSK:

```bash
sudo install -m 0600 -D /path/to/psk /run/the-machine/secrets/home-wifi
# MCP: net.connect_wifi(ssid: "MyNetwork", credential_ref: "home-wifi", token: ...)
```

---

## Audio

- `audio.list_devices` — `pactl list short` (PipeWire/PulseAudio)
- `audio.set_default` — `pactl set-default-sink` (grant token required)

PipeWire is included in the debootstrap rootfs package list.

---

## Hardware smoke checklist

Run on the **target machine** after install or from a live USB session:

```bash
bash build/hardware-smoke.sh          # software checks (also runs in CI)
```

Manual checks:

- [ ] `/dev/dri/card0` exists and `display.get_modes` returns EDID resolutions
- [ ] `evdev` nodes under `/dev/input/event*` deliver events to system-daemon
- [ ] `WAYLAND_DISPLAY` socket accepts a test client (`weston-simple-shm` or similar)
- [ ] `net.list_interfaces` shows your NIC; `net.set_interface_state` brings link up
- [ ] Wi-Fi connects with a credential file + `net.connect_wifi`
- [ ] `audio.list_devices` lists PipeWire sinks
- [ ] `systemctl start the-machine.target` brings up the full agent stack
- [ ] Reboot persists state under `/var/lib/the-machine`

---

## Troubleshooting

| Symptom | Check |
|---------|-------|
| Black screen | `THE_MACHINE_COMPOSITOR_BACKEND=drm`; confirm `/dev/dri/card0` |
| No Wayland clients | `XDG_RUNTIME_DIR=/run/the-machine`; compositor running |
| Wi-Fi fails | `wpa_supplicant` running; credential file mode `0600` |
| Mutations denied | Policy broker up; grant token from `policy.issue` |
| No audio | `pipewire` / `pipewire-pulse` services; `pactl info` |

---

## See also

- [Bare-metal architecture plan](../architecture/bare-metal-desktop.md)
- [System Daemon](../components/system-daemon.md)
- [Compositor](../components/compositor.md)
- [Getting Started](./getting-started.md)
