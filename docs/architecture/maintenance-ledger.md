# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
<<<<<<< HEAD
| audit system-daemon read-only MCP | cursor/maintenance-audit-system-daemon-readonly-mcp-e1cee1b | — | pr-open | 2026-08-30 | Handler tests for power/display/net/audio queries + net.connect_wifi guards |
=======
| policy-broker clippy | cursor/maintenance-audit-policy-broker-clippy-e1cee1b | — | pr-open | 2026-08-30 | audit: dead-code + clippy -D warnings |
>>>>>>> origin/main
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
<<<<<<< HEAD
| 2026-08-30 | audit | system-daemon read-only MCP handler tests | pr-open |
=======
| 2026-08-30 | audit | policy-broker clippy/dead-code | pr-open |
| 2026-08-30 | gap | G14 udev hotplug events | merged to main (#145) |
>>>>>>> origin/main
| 2026-08-30 | audit | hardware-smoke CI | merged to main (#143) |
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
1. **G17** — compositor: xdg-shell stub for third-party Wayland clients (cooldown until 2026-09-06)
2. **G13** — validate installed rootfs on bare metal (debootstrap + GRUB)
3. **audit** — event-bus MCP handler unit tests
4. **audit** — dead code / clippy in one crate (-p compositor)
=======
1. **G13** — merge #147/#148 (GRUB template CI) or operator hardware smoke
2. **G17** — compositor: wlroots seat/output (cooldown until 2026-09-06)
3. **audit** — missing MCP integration test in component-inventory.yaml
4. **audit** — `make verify` + fix first failure
>>>>>>> origin/main
