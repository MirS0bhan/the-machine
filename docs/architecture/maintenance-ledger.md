# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| verify-fix event-bus main.rs empty (build broken) | cursor/maintenance-verify-fix-event-bus-build-67ca1c0 | #198 | pr-open | 2026-08-30 | restore `event-bus/src/main.rs` + resolve conflict markers (rebased onto latest main, also fixed re-broken `component-inventory.yaml` conflict markers) |
| verify-fix initramfs modules skip | cursor/maintenance-verify-fix-initramfs-modules-9b9ce7c | #188 | merged | 2026-08-30 | skip test when /lib/modules missing |
| verify-fix ui-runtime merge conflict | cursor/maintenance-verify-fix-ui-runtime-conflict-dfabc51 | #189 | merged | 2026-08-30 | merge MCP handler + resolve_token unit tests |
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | verify-fix | `main` still broken after rebase: `event-bus/src/main.rs` empty + re-broken nested conflict markers in `component-inventory.yaml` (`integration_tests` block); rebased fix onto latest main via merge commit | pr-open (#198) |
| 2026-08-30 | verify-fix | ui-runtime merge conflict markers | merged to main (#189) |
| 2026-08-30 | audit | policy.register Python integration test | merged to main (#179) |
| 2026-08-30 | audit | event.subscribe Python integration test | merged to main (#182) |
| 2026-08-30 | verify-fix | test-initramfs-modules skip on missing /lib/modules | merged to main (#188) |
| 2026-08-30 | audit | lambda.deprecate Python integration test | merged to main (#176) |
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
2. **audit** — missing Python integration test for an MCP method (see `component-inventory.yaml`; many `system-daemon`/`ui-runtime`/`compositor`/`mcp-bus` methods still lack one)
3. **audit** — security pass: grant tokens / lambda entrypoint / external.register proxy rules
4. **docs** — `gap-analysis.md` "Open — Platform" table still lists G13 as open/"(in PR)"; all sub-items (debootstrap+GRUB, fstab, boot.auil, operator installed-rootfs validation) are merged to `main` (PRs #157, #159, #168) — move G13 to Closed in a tiny docs-only PR
5. **audit** — `make verify` + fix first failure
