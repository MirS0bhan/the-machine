# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| verify-fix ui-runtime merge conflict | cursor/maintenance-verify-fix-ui-runtime-conflict-4c83d6c | — | pr-open | 2026-08-30 | keep MCP handler tests + resolve_token tests |
| verify-fix initramfs modules skip | cursor/maintenance-verify-fix-initramfs-modules-9b9ce7c | #188 | pr-open | 2026-08-30 | skip module test when /lib/modules missing |
| audit event.schedule integration | cursor/maintenance-audit-event-schedule-integration-05da786 | #187 | pr-open | 2026-08-30 | event.schedule Python integration |
| audit marketplace.list integration | cursor/maintenance-audit-marketplace-list-integration-05da786 | #185 | pr-open | 2026-08-30 | marketplace.list Python integration |
| audit localmodel.complete integration | cursor/maintenance-audit-localmodel-complete-integration-05da786 | #183 | pr-open | 2026-08-30 | localmodel.complete integration |
| audit event.subscribe integration | cursor/maintenance-audit-event-subscribe-integration-05da786 | #182 | pr-open | 2026-08-30 | event.subscribe Python integration |
| audit policy.register integration | cursor/maintenance-audit-policy-register-integration-05da786 | #179 | pr-open | 2026-08-30 | policy.register Python integration |
| audit event-bus MCP handlers | cursor/maintenance-audit-event-bus-mcp-05da786 | #163 | pr-open | 2026-08-30 | duplicates also #180/#181 |
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | verify-fix | ui-runtime merge conflict markers | pr-open |
| 2026-08-30 | verify-fix | skip initramfs module test when /lib/modules missing | pr-open (#188) |
| 2026-08-30 | audit | event.schedule Python integration | pr-open (#187) |
| 2026-08-30 | gap | QEMU boot glibc + virtio-gpu + E2E proof | merged to main (#186) |
| 2026-08-30 | audit | marketplace.list Python integration | pr-open (#185) |
| 2026-08-30 | audit | CI lint job dedup | merged to main (#184) |
| 2026-08-30 | audit | localmodel.complete Python integration | pr-open (#183) |
| 2026-08-30 | audit | event.subscribe Python integration | pr-open (#182) |
| 2026-08-30 | audit | policy.register Python integration | pr-open (#179) |
| 2026-08-30 | audit | state.get/set/list policy-gated integration | merged to main (#178) |
| 2026-08-30 | audit | policy CONFIRM/HOLD Python integration | merged to main (#177) |
| 2026-08-30 | audit | lambda.deprecate Python integration | merged to main (#176) |
| 2026-08-30 | audit | lambda.search Python integration | merged to main (#175) |
| 2026-08-30 | verify-fix | maintenance-ledger conflict markers | merged to main (#170) |
| 2026-08-30 | audit | state-store clippy/dead-code | merged to main (#169) |
| 2026-08-30 | gap | G13 operator installed-rootfs validation | merged to main (#168) |
| 2026-08-30 | audit | local-model-daemon clippy/dead-code | merged to main (#167) |
| 2026-08-30 | audit | mcp-bus MCP handler unit tests | merged to main (#166) |
| 2026-08-30 | audit | lambda-server MCP handler unit tests | merged to main (#165) |
| 2026-08-30 | audit | local-model-daemon MCP handler unit tests | merged to main (#164) |
| 2026-08-30 | audit | event-bus MCP handler unit tests | pr-open (#163) |
| 2026-08-30 | audit | ui-runtime MCP handler unit tests | merged to main (#162) |
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
2. **audit** — missing Python integration test: `localmodel.classify_intent` / `localmodel.embed` / `event.register_handler` / `marketplace.install` / `policy.audit`
3. **audit** — security pass: grant tokens / lambda entrypoint / external.register proxy rules
4. **audit** — `make verify` + fix first failure
