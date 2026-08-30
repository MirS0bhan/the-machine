# Compositor

**Layer:** L5  
**Type:** Deterministic, non-LLM  
**Technology:** wlroots-based Wayland compositor  
**Language:** C or Rust  
**Dependencies:** System Daemon (for input events)  

---

## Overview

The Wayland Compositor in The Machine is a **standard-ish** compositor (based on wlroots) that can run conventional Wayland/X11 clients. Its role is to perform low-level compositing, damage tracking, frame scheduling, and input event delivery — all deterministic, all outside the agent's real-time path.

**Current implementation:** surfaces are painted via DRM/framebuffer/memory backends (`pixel.rs`). When `THE_MACHINE_COMPOSITOR_BACKEND=wayland`, the compositor binds a real `wl_display` socket (`wl_session.rs`) as the first G17 scaffold step toward a full wlroots session.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Wayland Compositor (wlroots-based)                              │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Input Forwarding                                          │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Receive     │  │ Route to    │  │ Deliver to          │ │ │
│  │  │ from System │  │ target      │  │ target              │ │ │
│  │  │ Daemon      │  │ surface     │  │ surface             │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Confirmation Surface                                       │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Protocol    │  │ Reserved    │  │ Exclusive           │ │ │
│  │  │ Extension   │  │ Role        │  │ Input               │ │ │
│  │  │             │  │ (only       │  │ Focus               │ │ │
│  │  │             │  │  Broker may │  │                     │ │ │
│  │  │             │  │  bind)      │  │                     │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Damage Tracking & Compositing                             │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Damage      │  │ Re-draw     │  │ Frame Scheduling    │ │ │
│  │  │ Regions     │  │ Affected    │  │ (vblank sync)       │ │ │
│  │  │             │  │ Surfaces    │  │                     │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Vibrancy / Blur                                           │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │ │
│  │  │ Blur        │  │ GPU        │  │ Composite           │ │ │
│  │  │ Region      │  │ Compute    │  │ Overlays            │ │ │
│  │  │ Management  │  │ (shaders)  │  │                     │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  XWayland                                                   │ │
│  │  ┌─────────────────────────────────────────────────────────┐ │ │
│  │  │  X server compatibility (for legacy apps)               │ │ │
│  │  └─────────────────────────────────────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Design Goals

1. **Don't reinvent a compositor** — wlroots already solves compositing well
2. **XWayland is the escape hatch, not the target** — legacy apps are supported but not prioritized
3. **Exactly one protocol addition** — the confirmation-surface role
4. **No opinion about AUIL/ASL** — it renders whatever the UI Runtime provides

---

## Input Forwarding

### Input Path

```
Keyboard → evdev → System Daemon → Compositor → Target Surface
```

The input path is **entirely deterministic** — no LLM in the critical path.

### Input Routing

The compositor routes input events to the correct surface based on:

1. **Keyboard:** Surface with keyboard focus
2. **Pointer:** Surface under the pointer
3. **Touch:** Surface under the touch point
4. **Confirmation surface:** While active, all input goes to the confirmation surface

### Provenance Marker

The compositor receives input events with ProvenanceMarkers from the System Daemon:

1. **For regular surfaces:** The marker is ignored
2. **For confirmation surface:** The marker is **verified** before input is accepted
   - HMAC is recomputed using the boot-time secret
   - If the marker is invalid, the input is dropped and logged

---

## Confirmation Surface

### Protocol Extension

The compositor implements a custom Wayland protocol extension:

```xml
<protocol name="confirmation_surface_v1" version="1">
  <interface name="zcr_confirmation_surface_v1" version="1">
    <request name="create">
      <arg name="id" type="new_id" interface="zcr_confirmation_surface_v1" />
      <arg name="surface" type="object" interface="wl_surface" />
    </request>
    <request name="show">
      <arg name="template_name" type="string" />
      <arg name="placeholders" type="array" />
    </request>
    <request name="destroy" type="destructor" />
    <event name="confirmed">
      <arg name="decision" type="uint" /> <!-- 0 = deny, 1 = allow -->
    </event>
  </interface>
</protocol>
```

### Enforcement

The compositor enforces the following rules for the confirmation surface:

1. **Exclusivity:** Only one confirmation surface can exist at a time
2. **Creator:** The `create` request is only accepted from the Confirmation Surface Daemon (verified by credential)
3. **Layer:** The confirmation surface is rendered at `ZWLR_LAYER_SHELL_V1_LAYER_OVERLAY` (above everything)
4. **Focus:** While the surface is mapped, it has exclusive keyboard/pointer focus
5. **Unfakeable:** No other client can bind to the confirmation surface role

### Lifecycle

```
┌─────────────────┐
│  IDLE           │
└────────┬────────┘
         │
         │ Broker calls show()
         ▼
┌─────────────────┐
│  MAPPED         │ (rendered above everything, exclusive input focus)
└────────┬────────┘
         │
         ├─ user confirms → confirmed(1)
         ├─ user denies → confirmed(0)
         ├─ timeout → confirmed(0)
         └─ destroy called → surface destroyed
         ▼
┌─────────────────┐
│  IDLE           │
└─────────────────┘
```

---

## Vibrancy / Backdrop-Blur

### Blur Region Protocol

The compositor supports a custom blur-region protocol (or uses the existing `zwp_blur_v1` protocol):

```xml
<protocol name="blur_v1" version="1">
  <interface name="zwp_blur_v1" version="1">
    <request name="create">
      <arg name="id" type="new_id" interface="zwp_blur_region_v1" />
      <arg name="surface" type="object" interface="wl_surface" />
    </request>
  </interface>
  <interface name="zwp_blur_region_v1" version="1">
    <request name="set_region">
      <arg name="x" type="int" />
      <arg name="y" type="int" />
      <arg name="width" type="int" />
      <arg name="height" type="int" />
      <arg name="radius" type="int" />
    </request>
    <request name="destroy" type="destructor" />
  </interface>
</protocol>
```

### Implementation

The blur is implemented using GPU compute shaders:

1. The UI Runtime declares a blur region
2. The compositor tracks the region on the surface
3. During compositing, the compositor applies a Gaussian blur to the region
4. The compositor uses GPU shaders for efficient blur computation

---

## Legacy App Support

### XWayland

The compositor includes XWayland support for running legacy X11 applications:

1. **XWayland server:** Runs as a separate process
2. **Surface mapping:** X11 windows are mapped to Wayland surfaces
3. **Input forwarding:** Input events are forwarded to the XWayland server
4. **Limitations:** XWayland apps are "second-class" — no vibrancy, no confirmation surface integration

### Embedding

The UI Runtime can embed XWayland surfaces using `ExternalSurface` nodes:

1. The UI Runtime creates an `ExternalSurface` node with the X11 window ID
2. The compositor uses `XReparentWindow` to embed the window
3. The window is positioned within the UI Runtime's surface

---

## Performance

### Targets

| Metric | Target |
|--------|--------|
| Input latency (keyboard → display) | < 20ms |
| Frame rate | 60 fps (minimum) |
| Damage tracking latency | < 1ms |
| Blur computation latency | < 2ms |
| Memory usage | < 200MB |

### Optimizations

1. **Damage tracking** — only re-draw damaged regions
2. **GPU compositing** — use GPU for compositing and blur
3. **Frame scheduling** — sync with vblank for tear-free rendering
4. **Surface caching** — cache rendered surfaces when they haven't changed

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `THE_MACHINE_COMPOSITOR_BACKEND` | `auto` | Pixel backend (`auto`, `drm`, `framebuffer`, `memory`) or `wayland` to bind `wl_display` |
| `THE_MACHINE_COMPOSITOR_SOCKET` | `/run/the-machine/compositor-input.sock` | Input socket from System Daemon |
| `THE_MACHINE_COMPOSITOR_OUTPUT` | `auto` | Output to use (`auto` detects first available) |
| `THE_MACHINE_COMPOSITOR_REFRESH` | `60` | Refresh rate in Hz |

### Command-Line Arguments

```
compositor [OPTIONS]

Options:
  --input-socket <PATH>     Input socket path
  --output <NAME>           Output to use (auto, HDMI-A-1, etc.)
  --refresh <HZ>            Refresh rate (default: 60)
  --xwayland                Enable XWayland support
  --help                    Show this help
```

---

## See Also

- [System Daemon](./system-daemon.md) — for input forwarding
- [UI Runtime](./ui-runtime.md) — for the primary client
- [Policy Broker](./policy-broker.md) — for the confirmation surface
- [MCP Bus](./mcp-bus.md) — for the message protocol
