# Wayland Compositor — Integration Spec (wlroots-based, minimal custom surface)

**Fills:** §3.6.1 of `agent-native-os-architecture.md` (Wayland Compositor)
**Related:** `system-daemon-spec.md` §2 (input forwarding this compositor receives), `ui-engine`'s existing runtime (the primary client this compositor serves), `policy-broker-spec.md` §9 (the one non-standard protocol extension this compositor must add)
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation — intentionally thin; most of this component is "use existing software correctly," not novel design

---

## 0. Design goals

1. **Don't reinvent a compositor.** Nothing about the agent-native premise requires new compositing, damage-tracking, or frame-scheduling logic — wlroots already solves this well, and inventing a replacement would be pure risk with no corresponding benefit to the OS's actual thesis. This spec's job is to say precisely *which* wlroots-based behavior this OS depends on and *what's* different, not to redesign compositing.
2. **XWayland is the escape hatch, not the target.** Parent §3.6.1 is explicit that conventional Wayland/X11 clients can still run for software not worth reimplementing as a lambda-backed component (the CAD-tool example). This spec treats that as a compatibility feature to preserve, not extend — no work here goes toward making legacy apps feel "native" to the agent, since they're explicitly outside its model.
3. **Exactly one protocol addition beyond stock wlroots: the confirmation-surface role.** This is the sole piece of genuinely custom compositor work this OS needs, and it's small and well-precedented (session-lock protocols already establish the "reserved surface role only one specific client may bind" pattern).
4. **The compositor has no opinion about AUIL/ASL.** It renders whatever the UI Runtime (`ui-engine`) hands it as ordinary Wayland surfaces/buffers — AUIL patch semantics, ASL tokens, and mixin resolution are entirely the UI Runtime's problem (`auil-asl-spec.md`), not something the compositor parses or understands. To the compositor, the UI Runtime is just a normal (if privileged) Wayland client.

---

## 1. Base: stock wlroots behavior, unmodified

- Standard compositing, damage tracking, frame scheduling, multi-output handling, DRM/KMS backend — no changes from upstream wlroots behavior.
- Standard `xdg-shell` for the UI Runtime's own surfaces, and standard `XWayland` support for legacy/escape-hatch clients (parent §3.6.1's CAD-tool case).
- Input event delivery from the System Daemon (`system-daemon-spec.md` §2) is accepted over that daemon's dedicated local channel, translated into the same internal event representation wlroots would use for a normal libinput-sourced event — from the compositor's internal perspective, input "comes from libinput" in the conventional sense; the System Daemon's separate ownership of the raw device (parent §3.1) is a system-architecture distinction that doesn't require the compositor's input-handling code to look different from a stock build.

---

## 2. Client roles

| Client | Role | Privilege |
|---|---|---|
| **UI Runtime** (`ui-engine`) | Renders the AUIL tree; primary/default client | Ordinary `xdg-shell` surfaces, but treated as the always-present "desktop" client (analogous to a shell/panel process in a conventional compositor setup) |
| **Legacy Wayland/X11 apps** | Escape hatch (parent §3.6.1) | Ordinary surfaces, composited alongside/within whatever the UI Runtime's tree currently allocates space for them — the UI Runtime is responsible for giving a legacy app's surface a place in the AUIL tree (e.g. via a `media`-like "external surface" primitive), not the compositor deciding layout on its own |
| **Confirmation Surface Daemon** (`policy-broker-spec.md` §9) | The one non-standard role (§3) | Reserved protocol role; no other client, including the UI Runtime, may bind it |

---

## 3. The confirmation-surface protocol extension

- A new Wayland protocol extension, `confirmation-surface-v1`, modeled directly on the existing `ext-session-lock-v1` pattern: a client requests the role, the compositor grants it to **at most one client for the lifetime of the compositor process**, identified by a fixed, compile-time-known socket/credential the Broker's Confirmation Surface Daemon connects with at its own startup (mirrors how session-lock implementations typically restrict the role to a specific, trusted binary).
- While a `confirmation-surface-v1` surface is mapped, the compositor:
  - Renders it above all other surfaces, full attention (analogous to a lock-screen surface taking exclusive focus).
  - Routes all input exclusively to it — no input event reaches the UI Runtime or any other client while a confirmation surface is active, which is what prevents a race where a UI-Runtime-rendered element could visually overlap or intercept a click meant for the confirmation surface.
  - Refuses any *other* client's request to bind the same role while one is already granted, and refuses the request outright if the requesting client's credential doesn't match the one fixed identity established at compositor startup.
- This is the entirety of the compositor's custom work. It does not need to understand *why* the surface exists, what a policy decision is, or anything about the Broker's rule language (`policy-broker-spec.md` §2) — it only needs to enforce "this role, this one client, exclusive input and top-most rendering while mapped."

---

## 4. Vibrancy / backdrop-blur handling

- `auil-asl-spec.md` §3.2 assigns `vibrancy=` token resolution to "compositor-level backdrop blur," not something the UI Runtime computes itself. Concretely: the UI Runtime's surfaces declare a blur region + intensity via a standard-shaped compositor protocol (e.g. an implementation of `wp_fractional_scale`-adjacent or a custom minimal blur-region protocol if no suitable stock one exists), and the compositor performs the actual blur compositing. This keeps the "looks native" property a compositor-layer guarantee rather than something every AUIL-authored surface has to reimplement, consistent with the design intent in `auil-asl-spec.md` §3.2.

---

## 5. Failure / degraded-mode behavior

- If the UI Runtime crashes or isn't yet started (very early boot, or Agent Core/local-model still warm-loading per `agent-core-spec.md` §8), the compositor continues running and displays whatever surface is currently mapped — most commonly, nothing but a background, or the Fallback Shell's own minimal client (`fallback-shell-spec.md`) if the UI Runtime is confirmed down rather than just not-yet-started. The compositor itself has no "agent unavailable" logic; that decision and indicator belong entirely to the Fallback Shell / UI Runtime, per parent §3.7.
- The compositor is not on the parent doc's protected-unit list by name, but restarting it mid-session is disruptive enough (it owns every visible surface) that it's a reasonable candidate for that list — this is flagged as an open item (§7) for the Policy Broker's protected-unit configuration rather than decided here.

---

## 6. Security summary

| Threat | Mitigation |
|---|---|
| A malicious client binds the confirmation-surface role to spoof a legitimate confirmation prompt | Role is granted to exactly one fixed, credential-verified client identity at compositor startup, never re-negotiated at runtime (§3) |
| A UI-Runtime-rendered element visually overlaps a real confirmation surface to trick the user into misclicking | Exclusive input routing and top-most compositing while the confirmation surface is mapped means no other surface can receive input or render above it during that window (§3) |
| Legacy XWayland app used as a side-channel to synthesize fake input toward the confirmation surface | Input exclusivity while a confirmation surface is mapped applies to every other client uniformly, XWayland included — there is no privileged input path for legacy apps |

---

## 7. Open items before implementation

1. **`confirmation-surface-v1` protocol spec** — this document describes the required behavior; the actual Wayland protocol XML and a wlroots implementation patch still need to be written.
2. **Blur-region protocol choice** (§4) — whether an existing stock protocol is adequate or a small custom one is needed; needs an actual survey of current wlroots protocol support before deciding.
3. **Compositor's place on the protected-unit list** — flagged in §5; needs a decision made alongside the Policy Broker's protected-unit configuration, not unilaterally here.
4. **External-surface embedding for legacy apps** (§2) — the exact mechanism by which the UI Runtime gives a legacy Wayland/XWayland surface a slot inside an AUIL tree needs its own small spec, likely as an addendum to `auil-asl-spec.md` rather than this document.
