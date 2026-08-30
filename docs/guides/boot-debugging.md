# Boot Debugging Guide

Use this when the screen stays blank after selecting **The Machine** in GRUB (QEMU or bare metal).

## Quick diagnosis (QEMU)

Rebuild the ISO, then boot with **serial logs on your terminal**:

```bash
make iso
make run-debug
```

Or manually (matches your command + logging):

```bash
qemu-system-x86_64 -accel kvm -m 2G \
  -cdrom build/the-machine.iso \
  -boot d -vga std \
  -serial mon:stdio
```

Pick **The Machine (debug)** from the GRUB menu. Boot messages appear on **ttyS0** (your terminal) and **tty0** (VGA window).

## What gets logged

| File | Contents |
|------|----------|
| `/var/log/boot.log` | Timestamped init milestones, display probe, service status |
| `/var/log/boot-report.txt` | Full snapshot: boot.log + tail of every `*.log` |
| `/var/log/compositor-backend` | Which pixel backend was chosen (`drm-kms`, `framebuffer`, or `memory`) |
| `/var/log/{service}.log` | Per-daemon stderr/stdout (`compositor`, `ui-runtime`, `agent-core`, …) |

### Reading logs inside the guest

From the **rescue shell** GRUB entry (`the-machine.rescue`):

```bash
cat /var/log/boot-report.txt
cat /var/log/compositor-backend
sh /collect-boot-logs.sh /tmp/report.txt
```

## GRUB menu entries

| Entry | Kernel cmdline | Use when |
|-------|----------------|----------|
| **The Machine** | `quiet` | Normal boot |
| **The Machine (debug)** | `the-machine.debug loglevel=7` | Verbose kernel + serial service tail |
| **The Machine (rescue shell)** | `the-machine.rescue` | Skip services; drop to `/bin/sh` immediately |

## Common blank-screen causes

### 1. Compositor on **memory** backend

If `/var/log/compositor-backend` contains `backend=memory`, pixels never reach the display. Check the display probe in `/var/log/boot.log`:

- `fb0: missing` — no legacy framebuffer (common in QEMU without proper VGA setup)
- `drm: /dev/dri missing` — no KMS device

**Fix:** ensure the kernel has VGA/DRM drivers (host kernel when building ISO), or force framebuffer:

```bash
# In /init before compositor starts (temporary test):
export THE_MACHINE_COMPOSITOR_BACKEND=framebuffer
```

### 2. DRM opens but scanout fails

Previously the compositor could select DRM even when `MODE_SETCRTC` failed (black screen, no error on VGA). It now **falls back** to `/dev/fb0` or memory and records the result in `/var/log/compositor-backend`.

### 3. `quiet` hides kernel panics

Use the **debug** GRUB entry or remove `quiet` from `build/mkiso.sh`.

### 4. Services crash before compositor

`boot_dump_status` in `/var/log/boot.log` lists each service as `running` or `NOT RUNNING` with log tails.

## Bare metal (USB stick)

1. Boot with **The Machine (debug)**.
2. Attach a USB-serial adapter or use a second machine on serial if available (`console=ttyS0,115200` is already on the cmdline).
3. If you can switch to a text console (Ctrl+Alt+F1 may not work without getty), logs are under `/var/log/`.
4. Reboot into **rescue shell** and run `cat /var/log/boot-report.txt`.

## Developer iteration (no ISO)

```bash
make qemu    # kernel + initramfs directly, nographic, debug cmdline
```

## Related files

- `build/boot-init.sh` — PID 1 service launcher
- `build/boot-log-lib.sh` — `boot_log`, `boot_probe_display`, `boot_dump_status`
- `build/collect-boot-logs.sh` — aggregate report script (also installed at `/collect-boot-logs.sh`)
- `build/mkiso.sh` — GRUB entries
- `compositor/src/pixel.rs` — writes `/var/log/compositor-backend`
