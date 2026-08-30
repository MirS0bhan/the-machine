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
| G12 lease fast-path | cursor/maintenance-g12-lease-fast-path-2392aba | — | pr-open | 2026-08-30 | Optional `THE_MACHINE_LEASE_FAST_PATH=1` binds relay socket |
| G14 display.set_mode DRM | cursor/maintenance-g14-display-mode-8d4febd | #136 | pr-open | 2026-08-30 | Prior agent run |
| G17 wl_display scaffold | — | #135 | merged | 2026-08-30 | `wl_session.rs` + wayland-server on main |
| G7 zbus D-Bus | — | — | merged | 2026-08-30 | Landed on main `f446927` |
| Policy fail-closed | — | — | merged | 2026-08-30 | Landed on main `ad7520d` |

## Recent runs (newest first)

| Date (UTC) | Run type | Target | Outcome |
|------------|----------|--------|---------|
| 2026-08-30 | gap | G12 lease fast-path | pr-open |
| 2026-08-30 | gap | G17 wl_display scaffold | merged to main |
| 2026-08-30 | gap | G7 | merged to main |
| 2026-08-30 | gap | policy hardening | merged to main |

## Next suggested work (agent-maintained)

Rotate when completing a row. Prefer top item not in cooldown.

1. **G14** — system-daemon: net.set_interface_state via rtnetlink (display path in PR #136)
2. **G17** — compositor: wlroots seat/output init (cooldown until 2026-09-06; scaffold merged)
3. **G12** — mcp-bus: full lease relay hardening / integration test (optional socket landed)
4. **audit** — `make verify` + fix first failure
5. **coverage** — lowest-coverage touched crate from `cargo llvm-cov` summary
