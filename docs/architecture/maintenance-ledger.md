# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| audit ui-runtime MCP handlers | cursor/maintenance-audit-ui-runtime-mcp-05da786 | #162 | pr-open | 2026-08-30 | ui.patch/get/tree/event/status/auil + unknown method |
| audit event-bus MCP handlers | cursor/maintenance-audit-event-bus-mcp-05da786 | #163 | pr-open | 2026-08-30 | event.publish/subscribe/schedule/register_handler |
| audit local-model-daemon MCP handlers | cursor/maintenance-audit-local-model-daemon-mcp-05da786 | #164 | pr-open | 2026-08-30 | localmodel.health/complete/classify_intent/embed |
| audit lambda-server MCP handlers | cursor/maintenance-audit-lambda-server-mcp-05da786 | #165 | pr-open | 2026-08-30 | lambda.register/invoke/search/deprecate |
| audit mcp-bus MCP handlers | cursor/maintenance-audit-mcp-bus-mcp-05da786 | #166 | pr-open | 2026-08-30 | bus.resolve/list_routes/_bus.register/deregister/lease/external |
| audit local-model-daemon clippy | cursor/maintenance-audit-local-model-daemon-clippy-05da786 | #167 | pr-open | 2026-08-30 | unused imports, dead_code |
| G13 operator rootfs validation | cursor/maintenance-g13-target-hw-validate-05da786 | #168 | pr-open | 2026-08-30 | installed-rootfs software checks |
| audit state-store clippy | cursor/maintenance-audit-state-store-clippy-05da786 | #169 | pr-open | 2026-08-30 | clippy/dead-code cleanup |
| verify-fix ledger conflict guard | cursor/maintenance-verify-fix-ledger-conflicts-05da786 | #170 | pr-open | 2026-08-30 | resolve markers + verify-docs guard |
| audit compositor clippy | cursor/maintenance-audit-compositor-clippy-05da786 | #171 | pr-open | 2026-08-30 | clippy/dead-code cleanup |
| audit event-bus clippy | cursor/maintenance-audit-event-bus-clippy-05da786 | #172 | pr-open | 2026-08-30 | clippy/dead-code cleanup |
| audit ui-runtime clippy | cursor/maintenance-audit-ui-runtime-clippy-05da786 | — | pr-open | 2026-08-30 | unused mut/vars, dead_code, private_interfaces |
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | audit | ui-runtime clippy/dead-code | pr-open |
| 2026-08-30 | audit | event-bus clippy/dead-code | pr-open (#172) |
| 2026-08-30 | audit | compositor clippy/dead-code | pr-open (#171) |
| 2026-08-30 | verify-fix | maintenance-ledger conflict markers | pr-open (#170) |
| 2026-08-30 | audit | state-store clippy/dead-code | pr-open (#169) |
| 2026-08-30 | gap | G13 operator installed-rootfs validation | pr-open (#168) |
| 2026-08-30 | audit | local-model-daemon clippy/dead-code | pr-open (#167) |
| 2026-08-30 | audit | mcp-bus MCP handler unit tests | pr-open (#166) |
| 2026-08-30 | audit | lambda-server MCP handler unit tests | pr-open (#165) |
| 2026-08-30 | audit | local-model-daemon MCP handler unit tests | pr-open (#164) |
| 2026-08-30 | audit | event-bus MCP handler unit tests | pr-open (#163) |
| 2026-08-30 | audit | ui-runtime MCP handler unit tests | pr-open (#162) |
| 2026-08-30 | audit | state-store MCP handler unit tests | merged to main (#161) |
| 2026-08-30 | audit | agent-core MCP handler unit tests | merged to main (#160) |
| 2026-08-30 | gap | G13 installer fstab for target HW | merged to main (#159) |
| 2026-08-30 | gap | G13 boot.auil in installed rootfs | merged to main (#157) |
| 2026-08-30 | audit | policy-broker MCP handler tests | merged to main (#156) |
| 2026-08-30 | audit | lambda-server clippy/dead-code | merged to main (#155) |
| 2026-08-30 | gap | boot greet e2e (GRUB → chat UI) | merged to main |
| 2026-08-30 | gap | G13 loopback installer GRUB | merged to main (#148) |
| 2026-08-30 | audit | compositor clippy/dead code | merged to main (#150) |
| 2026-08-30 | audit | fallback-shell MCP handler tests | merged to main (#153) |
| 2026-08-30 | audit | marketplace MCP handler tests | merged to main (#154) |
| 2026-08-30 | audit | system-daemon read-only MCP handler tests | merged to main (#149) |
| 2026-08-30 | audit | system-daemon MCP handler tests | merged to main (#146) |
| 2026-08-30 | gap | G14 udev hotplug events | merged to main (#145) |
| 2026-08-30 | audit | hardware-smoke CI | merged to main (#143) |
| 2026-08-30 | gap | compositor MCP handler tests | merged to main (#142) |
| 2026-08-30 | gap | G14 wifi wpa_cli connect | merged to main |
| 2026-08-30 | gap | G14 display.set_mode DRM | merged to main |
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
2. **audit** — mcp-bus clippy/dead-code cleanup
3. **audit** — missing Python integration test for an MCP method (see `component-inventory.yaml`)
4. **audit** — security pass: grant tokens / lambda entrypoint / external.register proxy rules
5. **audit** — `make verify` + fix first failure
