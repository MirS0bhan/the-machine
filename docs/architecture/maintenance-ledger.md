# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| G14 wifi connect | cursor/maintenance-g14-wifi-wpa-2baba1e | — | pr-open | 2026-08-30 | wpa_cli + owner-only credential files |
| verify-fix busybox/cpio | — | — | merged | 2026-08-30 | fetch-busybox.sh + busybox cpio fallback |
| G14 power.set_profile | — | — | merged | 2026-08-30 | cpufreq scaling_governor read/write |
| G17 wl_display scaffold | — | — | merged | 2026-08-30 | `wl_session.rs` + wayland-server on main |
| G12 lease fast-path | — | — | merged | 2026-08-30 | `THE_MACHINE_LEASE_FAST_PATH=1` relay socket |
| G13 rootfs kernel | — | — | merged | 2026-08-30 | debootstrap chroot kernel + `/vmlinuz` link |
| G14 display + net (partial) | — | — | merged | 2026-08-30 | display.rs + net sysfs on main |
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | gap | G14 wifi wpa_cli connect | pr-open |
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
2. **G14** — PipeWire default device + netlink admin (wifi connect wired)
3. **G13** — validate installed rootfs on bare metal (debootstrap + GRUB)
4. **audit** — `make verify` + fix first failure
