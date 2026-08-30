# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
<<<<<<< HEAD
| 2026-08-30 | audit | hardware-smoke CI | merged to main |
=======
| 2026-08-30 | gap | G14 udev hotplug events | merged to main |
| 2026-08-30 | audit | hardware-smoke CI | merged to main (#143) |
>>>>>>> cursor/merge-pr145-770f
| 2026-08-30 | audit | system-daemon MCP handler tests | merged to main (#146) |
| 2026-08-30 | gap | G14 wifi wpa_cli connect | merged to main |
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

<<<<<<< HEAD
1. **G14** — udev hotplug → event.publish (PR #145)
2. **G17** — compositor: wlroots seat/output + `wl_compositor` global (cooldown until 2026-09-06)
3. **G13** — validate installed rootfs on bare metal (debootstrap + GRUB)
4. **audit** — dead code / clippy in one crate (-p compositor)
=======
1. **G17** — compositor: wlroots seat/output + `wl_compositor` global (cooldown until 2026-09-06)
2. **G13** — validate installed rootfs on bare metal (debootstrap + GRUB)
3. **audit** — dead code / clippy in one crate (-p compositor)
4. **audit** — `make verify` + fix first failure
>>>>>>> cursor/merge-pr145-770f
