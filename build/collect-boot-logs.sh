#!/usr/bin/env bash
# Collect boot diagnostics from a running initramfs (or chroot).
# Usage (on the machine):  sh /collect-boot-logs.sh [output-file]
set -euo pipefail

OUT="${1:-/var/log/boot-report.txt}"

if [ -f /boot-log-lib.sh ]; then
  # shellcheck source=/dev/null
  . /boot-log-lib.sh
  boot_collect_logs "$OUT"
elif [ -f /var/log/boot.log ]; then
  {
    echo "The Machine boot report (manual collect)"
    date -u
    echo
    cat /var/log/boot.log
    echo
    for f in /var/log/*.log; do
      echo "=== $(basename "$f") ==="
      tail -50 "$f"
      echo
    done
  } >"$OUT"
else
  echo "No boot logs found. Is this a The Machine initramfs?" >&2
  exit 1
fi

echo "Wrote $OUT"
if [ -f /var/log/compositor-backend ]; then
  echo "--- compositor backend ---"
  cat /var/log/compositor-backend
fi
