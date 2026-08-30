# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| audit localmodel.classify_intent integration | cursor/maintenance-audit-localmodel-classify-intent-df6ea6f | — | pr-open | 2026-08-30 | Python integration test + inventory |
| verify-fix ui-runtime merge conflict | cursor/maintenance-verify-fix-ui-runtime-conflict-dfabc51 | #189–#192 | merged | 2026-08-30 | Landed on main (also #193 tree repair) |
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | audit | localmodel.classify_intent Python integration test | pr-open |
| 2026-08-30 | verify-fix | agent-core / event-bus / docs merge corruption | merged to main (#193) |
| 2026-08-30 | verify-fix | ui-runtime merge conflict markers | merged to main (#189–#192) |
| 2026-08-30 | audit | policy.register Python integration test | merged to main (#179) |
| 2026-08-30 | audit | event.subscribe Python integration test | merged to main (#182) |
| 2026-08-30 | verify-fix | test-initramfs-modules skip on missing /lib/modules | merged to main (#188) |
| 2026-08-30 | audit | lambda.deprecate Python integration test | merged to main (#176) |
| 2026-08-30 | gap | G13 operator installed-rootfs validation | merged to main (#168) |
| 2026-08-30 | audit | local-model-daemon clippy/dead-code | merged to main (#167) |
| 2026-08-30 | audit | mcp-bus MCP handler unit tests | merged to main (#166) |
| 2026-08-30 | audit | lambda-server MCP handler unit tests | merged to main (#165) |
| 2026-08-30 | audit | local-model-daemon MCP handler unit tests | merged to main (#164) |
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
2. **audit** — `localmodel.embed` or `event.register_handler` or `marketplace.install` Python integration test
3. **audit** — security pass: grant tokens / lambda entrypoint / external.register proxy rules
4. **docs** — move G13 to Closed in gap-analysis (all sub-items already on main)
