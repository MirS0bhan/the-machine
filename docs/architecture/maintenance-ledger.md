# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## Rules

- One **in-flight** row per gap ID. Do not start G7 again if a row exists for G7 unless status is `merged` or `abandoned`.
- Status values: `in-progress` | `pr-open` | `merged` | `abandoned` | `no-op`
- After merge to `main`, set status `merged` and add the date.
- Cooldown: do not re-open the same gap ID within 7 days of `merged` unless `make verify` proves a regression.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| verify-fix busybox/cpio | — | — | merged | 2026-08-30 | fetch-busybox.sh + busybox cpio fallback |
| G17 wl_display scaffold | — | — | merged | 2026-08-30 | `wl_session.rs` + wayland-server on main |
| G12 lease fast-path | — | — | merged | 2026-08-30 | `THE_MACHINE_LEASE_FAST_PATH=1` relay socket |
| G13 rootfs kernel | — | — | merged | 2026-08-30 | debootstrap chroot kernel + `/vmlinuz` link |
| G14 display + net (partial) | — | — | merged | 2026-08-30 | display.rs + net sysfs on main |
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | verify-fix | initramfs busybox/cpio fetch | merged to main |
| 2026-08-30 | gap | G13 kernel scaffold | merged to main |
| 2026-08-30 | gap | G12 lease fast-path | merged to main |
| 2026-08-30 | gap | bare-metal desktop A–C | merged to main |
| 2026-08-30 | gap | G17 wl_display scaffold | merged to main |
| 2026-08-30 | gap | G7 | merged to main |
| 2026-08-30 | gap | policy hardening | merged to main |

## Next suggested work (agent-maintained)

Rotate when completing a row. Prefer top item not in cooldown.

1. **G17** — compositor: wlroots seat/output + `wl_compositor` global
2. **G14** — wpa_supplicant wifi connect + PipeWire default device
3. **G13** — validate installed rootfs on bare metal (debootstrap + GRUB)
4. **audit** — `make verify` + fix first failure
