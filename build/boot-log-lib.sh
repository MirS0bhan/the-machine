#!/bin/sh
# Boot logging helpers for The Machine initramfs (busybox /bin/sh).
# Sourced by /init — do not execute directly.

BOOT_LOG=/var/log/boot.log

boot_ts() {
  date -u +%H:%M:%S 2>/dev/null || echo "??:??:??"
}

# Write to boot.log, VGA console (tty0), and serial (ttyS0).
boot_log() {
  line="[boot $(boot_ts)] $*"
  echo "$line" >>"$BOOT_LOG"
  echo "$line" >/dev/console 2>/dev/null || true
  echo "$line" >/dev/ttyS0 2>/dev/null || true
}

boot_log_section() {
  boot_log "=== $* ==="
}

boot_read_cmdline() {
  cat /proc/cmdline 2>/dev/null || true
}

boot_cmdline_has() {
  case " $(boot_read_cmdline) " in
    *" $1 "*) return 0 ;;
    *) return 1 ;;
  esac
}

boot_debug_enabled() {
  boot_cmdline_has "the-machine.debug" \
    || boot_cmdline_has "single" \
    || boot_cmdline_has "the-machine.rescue"
}

boot_probe_system() {
  boot_log_section "system probe"
  boot_log "cmdline: $(boot_read_cmdline)"
  boot_log "kernel: $(uname -r 2>/dev/null || echo unknown)"
  boot_log "arch: $(uname -m 2>/dev/null || echo unknown)"
  boot_log "memory: $(grep MemTotal /proc/meminfo 2>/dev/null | tr -s ' ' || echo unknown)"
  boot_log "pid1: $$"
  case "$(uname -r 2>/dev/null)" in
    *-azure|*-aws|*-gcp)
      boot_log "WARN: cloud-tuned kernel — QEMU -vga std often has no fb0/dri (blank screen)"
      boot_log "WARN: rebuild ISO with generic kernel: KERNEL=/boot/vmlinuz-*-generic make iso"
      ;;
  esac
}

boot_warn_if_no_display() {
  if [ -e /dev/fb0 ] || [ -d /dev/dri ]; then
    return 0
  fi
  boot_log "WARN: no /dev/fb0 and no /dev/dri — compositor will use memory backend (blank VGA)"
}

boot_probe_display() {
  boot_log_section "display probe"
  if [ -d /dev/dri ]; then
    boot_log "drm: $(ls -l /dev/dri 2>&1 | tr '\n' ' ')"
  else
    boot_log "drm: /dev/dri missing"
  fi
  if [ -e /dev/fb0 ]; then
    boot_log "fb0: $(ls -l /dev/fb0 2>&1)"
    if [ -r /sys/class/graphics/fb0/name ]; then
      boot_log "fb0 driver: $(cat /sys/class/graphics/fb0/name 2>/dev/null)"
    fi
    if [ -r /sys/class/graphics/fb0/virtual_size ]; then
      boot_log "fb0 size: $(cat /sys/class/graphics/fb0/virtual_size 2>/dev/null)"
    fi
  else
    boot_log "fb0: missing (compositor may use memory backend — no VGA output)"
  fi
  if [ -d /sys/class/drm ]; then
    boot_log "drm sysfs: $(ls /sys/class/drm 2>/dev/null | tr '\n' ' ')"
  fi
}

boot_svc_running() {
  name="$1"
  ps | grep -v grep | grep -q "/the-machine/${name}"
}

boot_check_svc() {
  name="$1"
  if boot_svc_running "$name"; then
    boot_log "${name}: running"
  else
    boot_log "${name}: NOT RUNNING"
    if [ -f "/var/log/${name}.log" ]; then
      boot_log "${name} log (last 8 lines):"
      tail -8 "/var/log/${name}.log" 2>/dev/null | while read -r ln; do
        boot_log "  | $ln"
      done
    fi
  fi
}

boot_dump_status() {
  boot_log_section "service status"
  for s in system-daemon mcp-bus policy-broker state-store event-bus \
    lambda-server local-model-daemon marketplace agent-core \
    compositor ui-runtime fallback-shell; do
    boot_check_svc "$s"
  done
  if [ -f /var/log/compositor-backend ]; then
    boot_log "compositor-backend: $(tr '\n' ' ' </var/log/compositor-backend)"
  else
    boot_log "compositor-backend: (not written yet)"
  fi
  boot_probe_display
}

boot_collect_logs() {
  out="${1:-/var/log/boot-report.txt}"
  {
    echo "The Machine boot report"
    echo "generated: $(boot_ts) UTC"
    echo
    cat "$BOOT_LOG" 2>/dev/null
    echo
    echo "=== compositor-backend ==="
    cat /var/log/compositor-backend 2>/dev/null || echo "(missing)"
    echo
    for f in /var/log/*.log; do
      [ -f "$f" ] || continue
      echo "=== $(basename "$f") (last 40 lines) ==="
      tail -40 "$f" 2>/dev/null
      echo
    done
  } >"$out"
  boot_log "boot report written to $out"
}

# Stream service logs to serial after the stack has had time to start.
boot_serial_log_tail() {
  (
    sleep 18
    boot_log_section "live service log tail (serial)"
    tail -n 0 -f /var/log/compositor.log /var/log/ui-runtime.log \
      /var/log/agent-core.log /var/log/mcp-bus.log 2>/dev/null | while read -r ln; do
      echo "[svc] $ln" >/dev/ttyS0 2>/dev/null || true
    done
  ) &
}
