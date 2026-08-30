// ============================================================================
// Default theme: macOS HIG × GNOME/libadwaita (Adwaita) reference design.
//
// Values are sourced and explained in:
//   ui-engine/docs/design-research-macos-gnome-adwaita.md
//
// This file is loaded automatically by UIRuntime.load_default_theme()
// (see ui-engine/runtime.py) and can be overridden per-surface by calling
// runtime.load_asl(...) again with app-specific tokens/styles — ASL styles
// merge (last-applied-wins), so apps only need to redefine what differs.
// ============================================================================

// ---- Surface colors (adaptive light/dark) ---------------------------------
token surface.primary = adaptive(light:#FAFAFA dark:#242424)
token surface.secondary = adaptive(light:#FFFFFF dark:#1E1E1E)
token surface.card = adaptive(light:#FFFFFF dark:#303030)
token surface.tertiary = adaptive(light:#EBEBEB dark:#2A2A2A)
token surface.overlay = adaptive(light:#FFFFFF dark:#383838)

// ---- Accent ----------------------------------------------------------------
token accent.bg = adaptive(light:#3584E4 dark:#3584E4)
token accent.fg = adaptive(light:#FFFFFF dark:#FFFFFF)
token accent.standalone = adaptive(light:#1B68C6 dark:#78AADE)

// ---- Text --------------------------------------------------------------
token text.primary = adaptive(light:#000000 dark:#FFFFFF)
token text.secondary = adaptive(light:#6E6E6E dark:#9A9996)
token text.disabled = adaptive(light:#00000066 dark:#FFFFFF66)

// ---- Borders / semantic states ---------------------------------------------
token border.subtle = adaptive(light:#00000019 dark:#FFFFFF1F)
token border.strong = adaptive(light:#00000033 dark:#FFFFFF33)
token shadow.color = adaptive(light:#00000029 dark:#00000047)
token destructive.bg = adaptive(light:#E01B24 dark:#C01C28)
token success.bg = adaptive(light:#2EC27E dark:#26A269)
token warning.bg = adaptive(light:#E5A50A dark:#CD9309)

// ---- Scales -----------------------------------------------------------
// radius: sm/md/lg match the original docs/spec.md example; xs/xl/window/pill extend it.
scale radius: xs=4 sm=6 md=10 lg=16 xl=20 window=15 pill=999
scale space: xxs=2 xs=4 sm=8 md=12 lg=16 xl=20 xxl=24 xxxl=32
scale elev: e0=0 e1=1 e2=2 e3=4 e4=8

// ---- Motion -----------------------------------------------------------
motion snappy = spring(stiffness=300 damping=26)
motion gentle = spring(stiffness=180 damping=20)
motion adwaita = duration(200ms ease=ease-out-cubic)
motion press = duration(100ms ease=ease-out)

// ---- Base behavioral mixins ---------------------------------------------
style Surface
  bg=token:surface.primary
  fg=token:text.primary
  radius=r-md
  border=token:border.subtle

style Hoverable
  on:hover => bg=token:surface.card elev=e2 motion=adwaita
  on:idle => elev=e0 motion=adwaita

style Pressable
  on:press => scale=0.97 motion=press
  on:release => scale=1.0 motion=snappy

// ---- Composed widget mixins ------------------------------------------------
style Card
  bg=token:surface.card
  fg=token:text.primary
  radius=r-lg
  elev=e1
  border=token:border.subtle
  padding=s-lg

style PrimaryButton
  bg=token:accent.bg
  fg=token:accent.fg
  radius=r-sm
  padding=s-md
  on:hover => elev=e2 motion=adwaita
  on:press => scale=0.96 bg=token:accent.standalone motion=press

style IconBtn
  bg=token:surface.overlay
  fg=token:text.primary
  radius=r-pill
  padding=s-xs
  on:hover => bg=token:surface.card motion=adwaita
  on:press => scale=0.92 motion=press

style Field
  bg=token:surface.secondary
  fg=token:text.primary
  radius=r-sm
  border=token:border.subtle
  padding=s-sm
  on:focus => border=token:accent.bg elev=e1 motion=adwaita
  on:error => border=token:destructive.bg motion=press

style ListRow
  bg=token:surface.secondary
  fg=token:text.primary
  radius=r-xs
  padding=s-md
  on:hover => bg=token:surface.card motion=adwaita
  on:press => bg=token:surface.tertiary motion=press

style Sidebar
  bg=token:surface.tertiary
  fg=token:text.primary
  radius=r-xs
  vibrancy=sidebar

style Popover
  bg=token:surface.overlay
  fg=token:text.primary
  radius=r-xl
  elev=e4
  vibrancy=popover

style Toggle
  bg=token:border.strong
  radius=r-pill
  on:change => bg=token:accent.bg motion=adwaita

style Slider
  bg=token:border.strong
  fg=token:accent.bg
  radius=r-pill
  on:drag => motion=snappy
