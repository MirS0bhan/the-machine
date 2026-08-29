# System Daemon — Raw I/O Ownership & the Narrow Kernel-Parameter MCP Surface

**Fills:** §3.1 of `agent-native-os-architecture.md` (the "small System Daemon" described at L0)
**Related:** `policy-broker-spec.md` §5 (schema-validated kernel/systemd ops this daemon executes), `policy-broker-spec.md` §9 (physically-originated input path this daemon owns, reused for confirmation-surface input provenance), `mcp-bus-spec.md` §2 (`system-op` namespace, pre-populated from this daemon's fixed tool set)
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation

---

## 0. Design goals

1. **As little code as possible, running as early as possible.** This is the one component that has to exist before the Bus, the Broker, or anything MCP-shaped is up, because raw input has to flow to *something* the instant the kernel hands it over. Every design choice here favors "small enough to audit by reading it once" over "flexible."
2. **Mechanism only, no policy.** The Daemon never decides *whether* a kernel-parameter change should happen — that's the Broker's job (`policy-broker-spec.md` §5). It only executes a fixed, pre-approved operation once told to, and only ever offers operations that were already whitelisted at the OS-image level, not ones it could be reconfigured into at runtime.
3. **The real-time input path never gets slower because of this OS's other ambitions.** Keyboard/mouse/audio latency here has to match (or beat) a conventional Linux desktop's input path — this daemon is not where "agent-native" trades away basic responsiveness.
4. **The one place input provenance is physically true.** `policy-broker-spec.md` §9's confirmation-surface design leans on there being a hardware-to-software path that nothing above L0 can synthesize. This spec is where that guarantee actually originates — everything above just trusts it.

---

## 1. Component overview

- A small, non-LLM daemon written in Rust or C, started effectively at PID-1-adjacent priority, before the compositor, before the Lambda Server, before anything MCP-facing (see parent boot order and `agent-core-spec.md` §8's fuller boot sequence — this daemon is up before `policy-broker` even starts).
- Owns raw device access: `evdev`/`libinput` for keyboard/mouse/touch, ALSA/PipeWire for audio device enumeration and routing, DRM/KMS hotplug notifications for monitor connect/disconnect, and the network interface list.
- Two responsibilities, kept structurally separate inside the daemon so a change to one can't accidentally affect the other:
  1. **Input forwarding** (§2) — zero MCP involvement, pure kernel-to-compositor plumbing.
  2. **Kernel-parameter MCP surface** (§3) — the only part of this daemon that speaks MCP at all.

---

## 2. Input forwarding — the real-time path

- Raw input events (keydown/up, pointer motion, button state, audio buffer callbacks) are forwarded directly to the Wayland Compositor (`compositor-spec.md`) over a dedicated, non-MCP local channel — this is Design Commitment #2 from the parent doc made concrete: this path has no MCP framing, no Broker check, no possibility of an agent or lambda intercepting it, because it was never routed through anything that could.
- **This is also the physically-originated input path** `policy-broker-spec.md` §9 requires for confirmation-surface approval: an input event that arrived via this daemon's forwarding path carries a provenance marker (a kernel-timestamp + device-id pair, not a claim any software component asserts about itself) that the Confirmation Surface Daemon checks for. No software component above this layer — including the Agent Core — has a code path that produces an event carrying this marker without it actually having come from a physical device, because the marker is stamped by this daemon reading directly from the kernel's input subsystem, not by anything downstream re-asserting it.
- Audio/video buffer plumbing (playback, capture) similarly never touches this daemon's MCP surface — the Lambda Server's media-handling lambdas (parent §3.6.2's video player example) get GPU/audio access via the mediated device path described in the parent doc (§3.2.1), not via a call to this daemon; this daemon's involvement in media is limited to owning the hotplug/device-enumeration events that tell the rest of the system what hardware exists.

---

## 3. Kernel-parameter MCP surface

- A **fixed, versioned, closed set** of operations, matching `policy-broker-spec.md` §5's schema-validation requirement exactly — this daemon does not accept free-form parameter names or values under any circumstance, including from the Broker itself. If an operation isn't in the compiled-in table below, there is no code path that executes it, full stop; expanding this set requires an OS image update, not a runtime registration.

```
power.get_profile / power.set_profile(profile: balanced|performance|powersave)
display.get_modes(output) / display.set_mode(output, mode)
net.list_interfaces() / net.set_interface_state(iface, up|down)
net.get_wifi_status() / net.connect_wifi(ssid, credential_ref)   — credential_ref points into a
                                                                    Broker-gated secret store, never
                                                                    a raw password in the call
audio.list_devices() / audio.set_default(device_id, role: output|input)
```

- Every call arrives already carrying a Broker-issued grant token (`policy-broker-spec.md` §4) — this daemon verifies the token's signature and scope match the requested operation before executing, but does **not** itself re-implement policy logic; a token that's structurally valid but was issued for a different operation is rejected here as a second, cheap check, not as this daemon's own judgment call about whether the action is a good idea.
- Results and current values are read-only queries with no side effects (`power.get_profile`, `display.get_modes`, etc.) and require no grant token at all — read access to "what state is the hardware in" is not gated, only mutation is, mirroring the general shape of `state-store-spec.md`'s read/write asymmetry.

---

## 4. Boot behavior

- Starts immediately after the kernel/initramfs hand-off, before `policy-broker` in the boot order (`agent-core-spec.md` §8) — this is a deliberate exception to "everything above L2 talks through the Broker," because at this point in boot there is no Broker yet to talk through. The daemon's kernel-op surface (§3) simply refuses all mutating calls until it observes the Broker come up and establish its own connection (a one-time handshake, not a per-call check at this stage) — before that handshake, only the read-only queries and input-forwarding path are live.
- This means the very earliest boot UI (parent §4 step 3 — compositor renders whatever the State Tree holds) can already display real display/audio hardware state without waiting for the Broker or Agent Core, which is what makes the Fallback Shell (`fallback-shell-spec.md`) able to show real system status even in the earliest, most degraded boot state.

---

## 5. Security summary

| Threat | Mitigation |
|---|---|
| Free-form kernel writes requested by a compromised agent | No such call shape exists — only the fixed, compiled-in operation table is executable (§3), matching `policy-broker-spec.md` §5's schema-validation stance from the other side |
| Forged input events used to fake human confirmation | Provenance marker is stamped from direct kernel-input reads inside this daemon; no software path above it can produce the marker without a real physical event (§2) |
| Grant-token replay against a different operation | Token scope is checked against the actual requested operation at execution time, not just verified as "signed by the Broker" (§3) |
| Daemon itself compromised (it runs with real device access) | Minimized code surface and early, narrow startup are the primary mitigation — this spec deliberately doesn't add features to this component, since every added feature is added attack surface at the one layer that can't be sandboxed the way L1 lambdas are |

---

## 6. Open items before implementation

1. **Credential store for `net.connect_wifi`** — `credential_ref` needs a concrete secret-storage design; this spec assumes one exists but doesn't define it.
2. **Hotplug event schema** — the shape of monitor/audio-device hotplug notifications forwarded toward the compositor and Event Bus (`event-bus-spec.md` §1, `category: external`) needs to be nailed down precisely.
3. **Handshake protocol with the Policy Broker** (§4) — "observes the Broker come up" needs an actual mechanism (a well-known socket path the Broker connects to on startup, most likely), not just a description.
4. **Provenance marker format** — the exact structure of the kernel-timestamp + device-id pair (§2), and how the Confirmation Surface Daemon verifies it wasn't replayed from a captured earlier event.
