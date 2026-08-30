# Maintenance Ledger — Agent Run State

Shared state for scheduled maintenance agents. **Read this before selecting work.**
Update in the same PR as your change; never mark gaps closed in `gap-analysis.md`
until the PR is merged to `main`.

## In flight

| Gap / task | Branch | PR | Status | Started | Notes |
|------------|--------|-----|--------|---------|-------|
| G13 rootfs CI validate | — | — | merged | 2026-08-30 | `rootfs-validate.sh` + test in Makefile |
| G14 rtnetlink | — | — | merged | 2026-08-30 | `system-daemon/src/netlink.rs` |
| G17 surface paint | — | — | merged | 2026-08-30 | `wl_shm` + commit → pixel blit |
| bare-metal phases A–E | — | — | merged | 2026-08-30 | see `bare-metal-desktop.md` |

## Next suggested work

1. **G13** — smoke-test installed rootfs on physical hardware (debootstrap + GRUB boot)
2. **G17** — wlroots xdg-shell compositing (optional; SHM path done)
3. **audit** — `make verify` on release bundle
