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
| verify-fix busybox/cpio | cursor/maintenance-verify-busybox-fetch-2392aba | — | pr-open | 2026-08-30 | fetch-busybox.sh + busybox cpio fallback for make verify |
| G17 wl_display scaffold | — | — | merged | 2026-08-30 | `wl_session.rs` + wayland-server on main |
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | verify-fix | initramfs busybox/cpio fetch | pr-open |
| 2026-08-30 | gap | G17 wl_display scaffold | merged to main |
| 2026-08-30 | gap | G7 | merged to main |
| 2026-08-30 | gap | policy hardening | merged to main |

## Next suggested work (agent-maintained)

Rotate when completing a row. Prefer top item not in cooldown.

1. **G17** — compositor: wlroots seat/output init (blocked: G17 merged <7d; wait or sub-step after cooldown)
2. **G14** — system-daemon: display.set_mode DRM (PR #136 open)
3. **G12** — mcp-bus: lease fast-path relay socket (PR #137 open)
4. **G13** — build: rootfs kernel scaffold (PR #138 open)
5. **audit** — clippy dead_code pass on mcp-bus lease crate
