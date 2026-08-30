#!/bin/sh
# PID 1 for The Machine ISO initramfs.
set -e

export PATH=/bin:/sbin:/the-machine
export RUST_LOG="${RUST_LOG:-info}"
export THE_MACHINE_SOCKET_DIR=/run/the-machine
export XDG_RUNTIME_DIR=/run/the-machine
export WAYLAND_DISPLAY=wayland-0
export STATE_STORE_BACKEND=sled
export STATE_STORE_PATH=/var/the-machine/state
export THE_MACHINE_LAMBDA_DIR=/var/the-machine/lambdas
export LOCAL_MODEL_PATH=/models/machine-tiny.gguf
export THE_MACHINE_BOOT_AUIL=/etc/the-machine/boot.auil
export THE_MACHINE_COMPOSITOR_BACKEND="${THE_MACHINE_COMPOSITOR_BACKEND:-auto}"

# shellcheck source=boot-log-lib.sh
. /boot-log-lib.sh

mkdir -p /var/the-machine/state /var/the-machine/lambdas /models \
  /run/the-machine/secrets /etc/the-machine /var/log

mount -t proc proc /proc
mount -t sysfs sys /sys
mount -t devtmpfs dev /dev 2>/dev/null || true
mkdir -p /run/the-machine

boot_check_dynamic_linker() {
  if [ -f /the-machine/compositor ] && [ ! -f /lib64/ld-linux-x86-64.so.2 ] \
    && [ ! -f /lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 ]; then
    boot_log "WARN: dynamic linker missing in initramfs — services will fail with ': not found'"
    boot_log "WARN: rebuild ISO: make initramfs-release (bundles libc from host)"
  fi
}

boot_log_section "The Machine boot starting"
boot_probe_system
boot_check_dynamic_linker

if boot_cmdline_has "the-machine.rescue"; then
  boot_log "the-machine.rescue on cmdline — dropping to /bin/sh"
  boot_collect_logs /var/log/boot-report.txt
  exec /bin/sh
fi

start_svc() {
  name="$1"
  path="/the-machine/$name"
  if [ ! -f "$path" ]; then
    boot_log "MISSING binary: $path"
    return
  fi
  boot_log "starting $name"
  "$path" >>"/var/log/$name.log" 2>&1 &
  echo "$!" >"/run/the-machine/${name}.pid"
}

# L0
start_svc system-daemon
sleep 1

# L3 (bus before broker consumers)
start_svc mcp-bus
sleep 1

# L2
start_svc policy-broker
sleep 1

# L1
start_svc state-store
start_svc event-bus
start_svc lambda-server
start_svc local-model-daemon
start_svc marketplace
sleep 1

# L4
start_svc agent-core
sleep 1

boot_probe_display

# QEMU -vga std may register /dev/fb0 slightly after devtmpfs mount.
fb_wait=0
while [ ! -e /dev/fb0 ] && [ "$fb_wait" -lt 3 ]; do
  sleep 1
  fb_wait=$((fb_wait + 1))
done

boot_warn_if_no_display

# Prefer legacy framebuffer when available (more reliable than DRM in QEMU).
if [ -e /dev/fb0 ]; then
  export THE_MACHINE_COMPOSITOR_BACKEND=framebuffer
  boot_log "using THE_MACHINE_COMPOSITOR_BACKEND=framebuffer (/dev/fb0 present)"
fi

# L5 — display session: compositor first, then UI, then shell
start_svc compositor
sleep 2
start_svc ui-runtime
sleep 2
start_svc fallback-shell

boot_log "boot stack launched — waiting for compositor backend"
sleep 3
boot_dump_status
boot_collect_logs /var/log/boot-report.txt

if boot_debug_enabled; then
  boot_log "debug mode: service logs also mirrored to serial (see ttyS0)"
  boot_serial_log_tail
fi

boot_log "boot complete — VGA: tty0  serial: ttyS0,115200"
boot_log "if screen is blank: cat /var/log/boot-report.txt or /var/log/compositor-backend"
boot_log "rescue shell: reboot, pick debug menu, add 'the-machine.rescue' to cmdline"

if boot_cmdline_has "single"; then
  boot_log "single on cmdline — dropping to /bin/sh after service start"
  exec /bin/sh
fi

# Keep PID 1 alive; services run in background.
while true; do sleep 3600; done
