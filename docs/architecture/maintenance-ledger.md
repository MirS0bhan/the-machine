# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| audit local-model-daemon clippy | cursor/maintenance-audit-local-model-daemon-clippy-05da786 | — | pr-open | 2026-08-30 | unused imports + mut cleanup |
| audit mcp-bus MCP handlers | cursor/maintenance-audit-mcp-bus-mcp-05da786 | #166 | pr-open | 2026-08-30 | bus.resolve, _bus.register, bus.lease |
| audit lambda-server MCP handlers | cursor/maintenance-audit-lambda-server-mcp-05da786 | #165 | pr-open | 2026-08-30 | lambda.register/invoke/search |
| audit local-model-daemon MCP handlers | cursor/maintenance-audit-local-model-daemon-mcp-05da786 | #164 | pr-open | 2026-08-30 | localmodel.health/complete/classify |
| audit event-bus MCP handlers | cursor/maintenance-audit-event-bus-mcp-05da786 | #163 | pr-open | 2026-08-30 | event.publish/subscribe/schedule |
| audit ui-runtime MCP handlers | cursor/maintenance-audit-ui-runtime-mcp-05da786 | #162 | pr-open | 2026-08-30 | ui.patch/get/status/auil.parse |
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | audit | local-model-daemon clippy/dead-code | pr-open |
| 2026-08-30 | audit | state-store MCP handler unit tests | merged (#161) |
| 2026-08-30 | audit | agent-core MCP handler unit tests | merged (#160) |
| 2026-08-30 | audit | policy-broker MCP handler tests | merged (#156) |
| 2026-08-30 | gap | G13 installer fstab for target HW | merged (#159) |
| 2026-08-30 | gap | G13 boot.auil in installed rootfs | merged (#157) |
| 2026-08-30 | audit | lambda-server clippy/dead-code | merged (#155) |
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
2. **G13** — operator target-HW validation on installed rootfs (software checks complete)
3. **audit** — missing MCP integration test in component-inventory.yaml
4. **audit** — `make verify` + fix first failure
