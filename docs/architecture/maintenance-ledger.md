# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| audit compositor MCP tests | cursor/maintenance-audit-compositor-mcp-7c1e350 | — | pr-open | 2026-08-30 | unit tests for compositor.present/surface/confirmation |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | audit | compositor MCP handler tests | pr-open |
| 2026-08-30 | gap | G17 surface paint | merged to main |
| 2026-08-30 | gap | G14 rtnetlink | merged to main |
| 2026-08-30 | gap | G13 rootfs CI validate | merged to main |

## Next suggested work

1. **G17** — wlroots xdg-shell compositing (cooldown until 2026-09-06; SHM path done)
2. **G13** — smoke-test installed rootfs on physical hardware (debootstrap + GRUB boot)
3. **audit** — clippy cleanup in `lambda-server` sandbox.rs
