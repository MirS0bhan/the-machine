# System Daemon

**Layer:** L0  
**Type:** Deterministic, non-LLM  
**Language:** Rust or C  
**PID:** 1-adjacent (started very early in boot)  

---

## Overview

The System Daemon is the **only component that exists before the MCP Bus, Policy Broker, or anything MCP-shaped is up**. It owns raw I/O (keyboard, mouse, audio, monitor hotplug events) and forwards input events to the compositor at native latency. It also exposes a minimal, versioned MCP interface for the few kernel parameters the OS actually needs to touch.

---

## Responsibilities

1. **Raw input ownership** — receive events from evdev/libinput and forward to compositor
2. **Kernel-parameter MCP surface** — expose a narrow, schema-validated subset of sysctl-like operations
3. **Hotplug monitoring** — track device additions/removals via udev
4. **Input provenance marking** — attach cryptographic markers to physically-originated events

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  System Daemon — single-threaded event loop (epoll/io_uring)    │
│                                                                   │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────────┐ │
│  │ Input       │  │ Kernel-op   │  │ Hotplug Monitor           │ │
│  │ Forwarder   │  │ Handler     │  │ (udev/listen)             │ │
│  │ (evdev)     │  │ (MCP)       │  │                           │ │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬──────────────┘ │
│         │                │                      │               │
│  ┌──────▼────────────────▼──────────────────────▼──────────────┐ │
│  │  Shared State: power_profile, display_modes, network_interfaces│ │
│  │               audio_devices, provenance_counter               │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Unix Sockets:                                              │ │
│  │    /run/the-machine/system-daemon.sock  (MCP + raw input)   │ │
│  │    /run/the-machine/compositor-input.sock  (input only)     │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Input Forwarding

### Event Loop Priorities

The System Daemon uses a strict priority-based event loop:

1. **Input forwarding** (highest priority) — read from evdev file descriptors, write to compositor socket
   - Never blocks
   - Uses non-blocking I/O
   - Processes events in the order they arrive

2. **Hotplug notifications** — udev events queued, processed in background
   - Lower priority than input forwarding
   - Updates internal device state

3. **MCP command processing** (lowest priority) — handled only when input queue is empty
   - Kernel-parameter operations
   - Status queries

### Provenance Marker Format

Every input event carries a **ProvenanceMarker** that proves it originated from physical hardware:

```rust
struct ProvenanceMarker {
    /// Monotonic clock timestamp at event capture (CLOCK_MONOTONIC)
    kernel_timestamp: u64,
    
    /// Device major/minor numbers from stat(2)
    device_major: u32,
    device_minor: u32,
    
    /// Monotonic per-device event counter
    sequence: u64,
    
    /// HMAC-SHA256 of (timestamp + device_major + device_minor + sequence)
    /// Key: boot-time secret, never exposed via MCP
    hmac: [u8; 32],
}
```

The HMAC secret is generated at boot using `/dev/urandom` and stored only in memory. It is **never** exposed via MCP or any other interface.

### Forwarding Channel

- **Socket type:** Unix domain socket (`SOCK_SEQPACKET`)
- **Path:** `/run/the-machine/compositor-input.sock`
- **Maximum message size:** 64KB
- **Protocol:** Raw binary with length prefix

The compositor reads from this socket in its own event loop. Any message that fails framing is dropped and logged.

### Supported Input Devices

| Device Type | evdev Code | Forwarded As |
|-------------|------------|--------------|
| Keyboard | EV_KEY | Keyboard event |
| Mouse | EV_REL, EV_ABS | Pointer event |
| Touchpad | EV_ABS | Pointer event |
| Touchscreen | EV_ABS | Pointer event |
| Gamepad | EV_KEY, EV_ABS | Gamepad event |

---

## Kernel-Op Handler

### State Machine

```
┌─────────────────┐
│ WAITING_FOR_    │
│ BROKER          │
└────────┬────────┘
         │
         │ hello from Broker
         ▼
┌─────────────────┐
│ READY           │
└────────┬────────┘
         │
         │ MCP call with grant token
         ▼
┌─────────────────┐
│ PROCESSING      │
│ (validate token,│
│  execute op)    │
└────────┬────────┘
         │
         ├─ success → return result
         │
         └─ failure → return error
              (state → READY)
```

### Handshake Protocol

1. Broker connects to `/run/the-machine/system-daemon.sock`
2. Broker sends: `{"type": "hello", "token": "<shared_secret>"}`
3. System Daemon validates the token (hard-coded in both binaries)
4. If valid, System Daemon enters READY state
5. Broker can now send MCP calls

The shared secret is compiled into both the System Daemon and Broker binaries at build time. It is unique per OS image.

### Operation Implementations

| MCP Method | Implementation | Notes |
|------------|----------------|-------|
| `power.get_profile` | Read `/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor` | Returns current profile |
| `power.set_profile` | Write to scaling_governor files | Requires grant token |
| `display.get_modes` | Use DRM/KMS `drmModeGetResources` + `drmModeGetConnector` | Returns available modes |
| `display.set_mode` | Use DRM/KMS `drmModeSetCrtc` | Requires grant token |
| `net.list_interfaces` | Use `rtnetlink` (RTM_GETLINK) | Returns all interfaces |
| `net.set_interface_state` | Use `rtnetlink` (RTM_SETLINK) | Requires grant token |
| `net.get_wifi_status` | Use `wpa_supplicant` D-Bus API | Returns connection status |
| `net.connect_wifi` | Use `wpa_supplicant` D-Bus API | Credential ref only, never raw password |
| `audio.list_devices` | Use ALSA `snd_card_next` or PipeWire | Returns audio devices |
| `audio.set_default` | Update PipeWire or ALSA default symlink | Requires grant token |

### Grant Token Verification

For every mutation operation:

1. Extract the grant token from the MCP call
2. Verify the token signature using the Broker's public key (hard-coded in System Daemon)
3. Verify the token scope matches the requested operation exactly
4. Verify the token has not expired
5. If all checks pass, execute the operation

---

## Hotplug Monitoring

### udev Integration

The System Daemon monitors udev for device additions and removals:

```rust
struct HotplugEvent {
    action: HotplugAction,  // Add, Remove, Change
    subsystem: String,      // "input", "drm", "sound", "net"
    device_path: String,    // e.g., "/devices/pci0000:00/0000:00:14.0/usb1/1-1/1-1:1.0"
    device_node: Option<String>,  // e.g., "/dev/input/event12"
    properties: HashMap<String, String>,  // udev properties
}
```

### Handled Subsystems

| Subsystem | Action | System Daemon Response |
|-----------|--------|------------------------|
| input | Add | Open evdev device, add to epoll |
| input | Remove | Close evdev device, remove from epoll |
| drm | Add | Query new display modes, notify compositor |
| drm | Remove | Notify compositor of display removal |
| sound | Add | Query audio device, update list |
| sound | Remove | Update audio device list |
| net | Add | Query interface, update list |
| net | Remove | Update interface list |

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `THE_MACHINE_SOCKET_DIR` | `/run/the-machine` | Directory for Unix sockets |
| `THE_MACHINE_LOG_LEVEL` | `info` | Logging verbosity (error, warn, info, debug) |
| `THE_MACHINE_BROKER_TIMEOUT` | `30s` | Timeout for Broker handshake |

### Command-Line Arguments

```
system-daemon [OPTIONS]

Options:
  --socket-dir <PATH>       Socket directory (default: /run/the-machine)
  --log-level <LEVEL>      Log level (default: info)
  --broker-timeout <DUR>   Broker handshake timeout (default: 30s)
  --help                   Show this help
```

---

## Failure Modes

| Failure | Behavior | Recovery |
|---------|----------|----------|
| evdev device disappears | Log, remove from epoll, re-scan on hotplug | Automatic |
| Compositor socket full | Drop oldest queued input event, increment counter | Automatic, visible via stats |
| Broker handshake timeout | Retry every 5s, read-only queries work, mutations return error | Automatic retry |
| Malformed MCP frame | Close connection, log, require reconnect | Manual (Broker must reconnect) |
| Kernel operation fails | Return structured error to caller | Manual (caller must retry) |

---

## MCP Interface

### Methods

#### Read-Only (no grant token required)

```
power.get_profile() → {"profile": "balanced" | "performance" | "powersave"}

display.get_modes() → {"modes": [{"width": u32, "height": u32, "refresh": f32, "current": bool}, ...]}

display.get_current_mode() → {"width": u32, "height": u32, "refresh": f32}

net.list_interfaces() → {"interfaces": [{"name": string, "type": string, "state": string}, ...]}

audio.list_devices() → {"devices": [{"name": string, "type": "input" | "output", "default": bool}, ...]}

system-daemon.stats() → {
    "input_events_forwarded": u64,
    "input_events_dropped": u64,
    "kernel_ops_executed": u64,
    "kernel_ops_denied": u64,
    "broker_status": "connected" | "disconnected" | "handshake_timeout",
    "uptime": f64
}
```

#### Mutations (require grant token)

```
power.set_profile(profile: "balanced" | "performance" | "powersave") → {}

display.set_mode(width: u32, height: u32, refresh: f32) → {}

net.set_interface_state(name: string, state: "up" | "down") → {}

net.connect_wifi(ssid: string, credential_ref: string) → {"status": "connecting" | "connected" | "failed"}

audio.set_default(name: string) → {}
```

---

## Security Considerations

1. **No raw sysctl access** — the System Daemon only exposes a pre-approved, schema-validated subset of operations
2. **Input provenance** — every input event carries a cryptographic marker that proves it came from physical hardware
3. **Minimal attack surface** — the System Daemon has no network exposure, only Unix sockets
4. **Deterministic behavior** — no LLM, no probabilistic code paths
5. **Audit logging** — all kernel operations are logged to syslog with timestamps and caller identity

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Input event latency (keyboard) | < 1ms |
| Input event latency (mouse) | < 2ms |
| Hotplug detection latency | < 100ms |
| Kernel operation latency | < 10ms |
| Memory usage | < 10MB |

---

## See Also

- [Policy Broker](./policy-broker.md) — for capability enforcement
- [Compositor](./compositor.md) — for input rendering
- [MCP Bus](./mcp-bus.md) — for the message protocol
