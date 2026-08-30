# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| G14 udev hotplug | cursor/maintenance-g14-hotplug-events | — | pr-open | 2026-08-30 | kernel uevent → event.publish |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | gap | G14 udev hotplug events | pr-open |
| 2026-08-30 | gap | G14 display.set_mode DRM | merged to main |
| 2026-08-30 | gap | compositor MCP handler tests | merged to main (#142) |
| 2026-08-30 | gap | G14 power.set_profile sysfs | merged to main |
| 2026-08-30 | verify-fix | initramfs busybox/cpio fetch | merged to main |
| 2026-08-30 | gap | G13 kernel scaffold | merged to main |
| 2026-08-30 | gap | G12 lease fast-path | merged to main |
| 2026-08-30 | gap | bare-metal desktop A–C | merged to main |
| 2026-08-30 | gap | G17 wl_display scaffold | merged to main |
| 2026-08-30 | gap | G7 | merged to main |
| 2026-08-30 | gap | policy hardening | merged to main |

## Next suggested work

Rotate when completing a row. Prefer top item not in cooldown.

1. **G17** — compositor: wlroots seat/output + `wl_compositor` global (cooldown until 2026-09-06)
2. **G13** — validate installed rootfs on bare metal (debootstrap + GRUB)
3. **audit** — merge open PRs #143 (hardware-smoke CI) and #144 (system-daemon MCP tests)
4. **audit** — `make verify` + fix first failure
