# Boot Debugging Guide

Use this when the screen stays blank after selecting **The Machine** in GRUB (QEMU or bare metal).

## Quick diagnosis (QEMU)

Rebuild the ISO, then boot with **serial logs on your terminal**:

```bash
make iso
make run-debug
```

`run-debug` uses **virtio-vga** (not legacy `-vga std`) so the bundled `virtio-gpu` kernel module can create `/dev/fb0`. If you boot manually:

```bash
qemu-system-x86_64 -accel kvm -m 2G \
  -cdrom build/the-machine.iso \
  -boot d \
  -vga none -device virtio-vga \
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

### 0. **Cloud-tuned kernel in the ISO** (your log shows this)

Your serial log included:

```
Linux version 6.17.0-1022-azure ...
```

**Azure/AWS/GCP kernels are built for their hypervisor, not QEMU std-VGA.** The initramfs can start every service (as in your log) but the compositor has no `/dev/fb0` or `/dev/dri` → **memory backend** → blank screen. The line after `boot complete` is not a hang; PID 1 is sleeping while services run in the background.

**Fix:** rebuild the ISO with a generic kernel:

```bash
sudo apt install linux-image-generic   # Debian/Ubuntu
KERNEL=/boot/vmlinuz-$(uname -r | sed 's/-azure//;s/$/-generic/') make iso
# or explicitly:
ls /boot/vmlinuz-*-generic
KERNEL=/boot/vmlinuz-6.8.0-XX-generic make iso
```

`make iso` now prefers `*-generic` over `*-azure` automatically when both are installed.

### 1. Compositor on **memory** backend

If `/var/log/compositor-backend` contains `backend=memory`, pixels never reach the display. Check the display probe in `/var/log/boot.log`:

- `fb0: missing` — no legacy framebuffer (common in QEMU without virtio-vga or kernel modules)
- `drm: /dev/dri missing` — no KMS device

**Fix:** rebuild the ISO (initramfs bundles `virtio-gpu` + deps) and boot with virtio VGA:

```bash
qemu-system-x86_64 ... -vga none -device virtio-vga -serial mon:stdio
# or: make run-debug
```

Legacy `-vga std` needs `bochs.ko`, which is often **not packaged** in Ubuntu generic modules — prefer virtio-vga.

### 2. DRM opens but scanout fails

Previously the compositor could select DRM even when `MODE_SETCRTC` failed (black screen, no error on VGA). It now **falls back** to `/dev/fb0` or memory and records the result in `/var/log/compositor-backend`.

### 3. `quiet` hides kernel panics

Use the **debug** GRUB entry or remove `quiet` from `build/mkiso.sh`.

### 4. Services crash before compositor

`boot_dump_status` in `/var/log/boot.log` lists each service as `running` or `NOT RUNNING` with log tails.

#### `/the-machine/<service>: not found` (binary exists)

If every service log shows:

```
/init: line 41: /the-machine/system-daemon: not found
```

the ELF is present but the **dynamic linker** (`/lib64/ld-linux-x86-64.so.2`) is missing from the initramfs. Rust binaries are dynamically linked against glibc on the build host; busybox alone is not enough.

**Fix:** rebuild the initramfs/ISO — `build/mkinitramfs.sh` bundles shared libraries via `build/bundle-shared-libs.sh`. Verify with:

```bash
bash build/test-initramfs-libs.sh
make iso
```

At early boot, `/init` also warns when the linker is absent (`boot_check_dynamic_linker` in `build/boot-init.sh`).

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
- `build/mkinitramfs.sh` — assembles initramfs; bundles glibc via `build/bundle-shared-libs.sh`
- `build/bundle-shared-libs.sh` — copies `ld-linux`, `libc`, and other `ldd` deps into the initramfs
- `compositor/src/pixel.rs` — writes `/var/log/compositor-backend`
