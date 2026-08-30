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
| G17 wl_display scaffold | — | — | merged | 2026-08-30 | `wl_session.rs` + wayland-server on main |
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | gap | G17 wl_display scaffold | merged to main |
| 2026-08-30 | gap | G7 | merged to main |
| 2026-08-30 | gap | policy hardening | merged to main |

## Next suggested work (agent-maintained)

Rotate when completing a row. Prefer top item not in cooldown.

1. **G17** — compositor: wlroots seat/output init (wl_display scaffold merged)
2. **G14** — system-daemon: one mutation path (e.g. display mode sysfs) behind grant token
3. **G12** — mcp-bus: document lease fast-path honestly or bind optional lease socket
4. **audit** — `make verify` + fix first failure
5. **coverage** — lowest-coverage touched crate from `cargo llvm-cov` summary
