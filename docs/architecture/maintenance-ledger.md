# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
<<<<<<< HEAD
| audit state-store MCP handlers | cursor/maintenance-audit-state-store-mcp-dd029fb | — | pr-open | 2026-08-30 | state.get/set/patch/list/stats + unknown method |
=======
| audit agent-core MCP handler tests | cursor/maintenance-audit-agent-core-mcp-dd029fb | #160 | pr-open | 2026-08-30 | `agent.status`, `agent.cloud.status`, local_only_mode guards |
>>>>>>> origin/main
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
<<<<<<< HEAD
| 2026-08-30 | audit | state-store MCP handler unit tests | pr-open |
| 2026-08-30 | audit | lambda-server clippy/dead-code | merged (#155) |
=======
| 2026-08-30 | audit | agent-core MCP handler unit tests | pr-open |
| 2026-08-30 | audit | policy-broker MCP handler tests | merged to main (#156) |
| 2026-08-30 | gap | G13 installer fstab for target HW | merged to main (#159) |
| 2026-08-30 | gap | G13 boot.auil in installed rootfs | merged to main (#157) |
| 2026-08-30 | audit | lambda-server clippy/dead-code | merged to main (#155) |
>>>>>>> origin/main
| 2026-08-30 | gap | boot greet e2e (GRUB → chat UI) | merged to main |
| 2026-08-30 | gap | G13 loopback installer GRUB | merged to main (#148) |
| 2026-08-30 | audit | compositor clippy/dead code | merged to main (#150) |
| 2026-08-30 | audit | fallback-shell MCP handler tests | merged to main (#153) |
| 2026-08-30 | audit | marketplace MCP handler tests | merged to main (#154) |
| 2026-08-30 | audit | system-daemon read-only MCP handler tests | merged to main (#149) |
| 2026-08-30 | audit | policy-broker clippy/dead-code | merged to main (#152) |
| 2026-08-30 | gap | G14 udev hotplug events | merged to main (#145) |
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

1. **G17** — compositor: wlroots seat/output (cooldown until 2026-09-06)
<<<<<<< HEAD
2. **audit** — event-bus or ui-runtime MCP handler unit tests
3. **audit** — `make verify` + fix first failure
=======
2. **G13** — operator target-HW validation on installed rootfs (software checks complete)
3. **audit** — missing MCP integration test in component-inventory.yaml
4. **audit** — `make verify` + fix first failure
>>>>>>> origin/main
