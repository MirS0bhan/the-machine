# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| G14 display.set_mode DRM | cursor/maintenance-g14-display-mode-8d4febd | — | pr-open | 2026-08-30 | sysfs mode read + DRM SETCRTC mutation |
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | gap | G14 | pr-open — display.set_mode DRM path |
| 2026-08-30 | gap | G7 | merged to main |
| 2026-08-30 | gap | policy hardening | merged to main |

## Next suggested work

Rotate when completing a row. Prefer top item not in cooldown.

1. **G17** — compositor: wlroots seat/output init (G17 wl_display PRs #133–#135 in flight)
2. **G14** — system-daemon: netlink interface up/down or wifi connect (display.set_mode wired in PR)
3. **G12** — mcp-bus: document lease fast-path honestly or bind optional lease socket
4. **audit** — `make verify` + fix first failure
5. **coverage** — lowest-coverage touched crate from `cargo llvm-cov` summary
