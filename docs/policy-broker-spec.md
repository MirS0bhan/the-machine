# Policy Broker — Capability Enforcement, Policy Language, Confirmation & Audit

**Fills:** §3.3 of `agent-native-os-architecture.md` (Policy Broker) and §7.2 ("Broker policy language")
**Related:** `lambda-server-spec.md` §2 (CAPS power set, this doc is what actually evaluates it), `auil-asl-spec.md` §2.4 (component depth cap) and §8 (intent-registry registration), `agent-core-spec.md` §7 (protected-unit systemd confirmation)
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation

---

## 0. Design goals

1. **Boring on purpose.** Every other component in this OS is allowed to be probabilistic, generative, or novel. The Broker is none of those things. It is the one component in the stack that should be gate-checkable by formal methods, not evals.
2. **Deny by default, explain on denial.** Every rejection carries a machine-readable reason (which rule fired, what was missing) so the Agent Core can self-correct without a human in the loop for the common case, while a human is still required for the sensitive case.
3. **Provenance over content.** The Broker does not try to understand *what* a capability request means semantically — that's the agent's job. It only checks *whether the request is structurally permitted and whether it traces back to user intent*, never whether it "seems reasonable."
4. **One decision surface for every layer.** Kernel ops (parent §3.3), lambda capability manifests (`lambda-server-spec.md` §2), component/style definitions (`auil-asl-spec.md` §2.4), UI patches (`auil-asl-spec.md` §7), and intent-registry registrations (`auil-asl-spec.md` §8) all pass through the *same* four-way decision model (§3) and the *same* audit log (§7) — not five bespoke validators.
5. **A confirmation the agent cannot forge.** This spec exists in part to close the open problem the parent architecture left dangling: if a compromised or simply overconfident agent can render its own "are you sure?" dialog, that dialog isn't a security boundary — it's theater. §9 defines the fix.

---

## 1. Component map

```
┌──────────────────────────────────────────────────────────────────┐
│  Policy Broker (L2)                                               │
│                                                                    │
│  ┌────────────────┐  ┌─────────────────┐  ┌────────────────────┐ │
│  │ Rule Store      │  │ Decision Engine  │  │ Confirmation       │ │
│  │ (versioned      │  │ (evaluates a     │  │ Surface Daemon     │ │
│  │  policy docs,   │  │  request against │  │ (§9 — out-of-band, │ │
│  │  §2)            │  │  rules, §3)      │  │  protected render) │ │
│  └────────┬────────┘  └────────┬─────────┘  └──────────┬─────────┘ │
│           │                    │                        │           │
│  ┌────────▼────────────────────▼────────────────────────▼────────┐ │
│  │              Anomaly Detector + Rate Limiter (§6)               │ │
│  └────────────────────────────────┬────────────────────────────────┘ │
│                                    │                                  │
│  ┌─────────────────────────────────▼──────────────────────────────┐ │
│  │                 Immutable Audit Log (§7, append-only)            │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                                                                    │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │  MCP Control Interface (policy.check / .grant / .confirm / ...) │ │
│  └───────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

Every arrow into the Broker is an MCP call, same as everything else in this OS (parent §3.4). The Broker is itself capability-scoped and sandboxed like any other L1/L2 component — it has no special root access; its authority comes from being the *only* thing every other component is configured to obey, not from privilege.

---

## 2. Policy language

A policy is a versioned document, not a database row — the same "immutable version, roll back on failure" philosophy the parent doc applies to lambdas (§3.2.1) and this doc applies to rules.

```
policy lambda.capability-grant
  version: 4
  applies_to: lambda.register
  rule default-deny
    match: always
    decision: DENY
    reason: "no matching allow rule"

  rule pure-functions-auto-allow
    match: capabilities == [] or capabilities == "pure"
    decision: ALLOW

  rule scoped-network-auto-allow
    match: capabilities.CAP_NET_OUT.domains subset_of known_domains(function.description)
    decision: ALLOW

  rule sensitive-capability-confirm
    match: capabilities intersects [CAP_MIC, CAP_CAMERA, CAP_FS_WRITE, CAP_NET_OUT(domains=new)]
    decision: CONFIRM
    surface: confirmation-surface
    template: capability-grant-request

  rule anomalous-combination-hold
    match: capabilities.CAP_FS_WRITE and capabilities.CAP_CAMERA
    decision: HOLD
    reason: "unusual capability combination for stated function purpose"
```

- **`policy <name>`** — one policy document per decision surface (`lambda.capability-grant`, `kernel.sysctl-op`, `component.definition`, `ui.patch`, `mcp.intent-register`, `systemd.protected-action`). Each is independently versioned so tightening one surface never requires touching the others.
- **`applies_to`** — the MCP method(s) this policy governs. The Decision Engine (§3) dispatches purely by method name; it never inspects call content to decide *which* policy applies, only *what the policy says* once selected.
- **`rule`** blocks are evaluated top-to-bottom, first match wins, with an implicit `default-deny` rule mandatory at the top of every policy document — the Broker refuses to load a policy document that omits one. This mirrors the CAPS power-set philosophy in `lambda-server-spec.md` §2.1: nothing is permitted that isn't explicitly named.
- **`match`** expressions are a small, closed predicate language (boolean combinators, set membership, `subset_of`, `intersects`) over a fixed schema of request fields — deliberately not Turing-complete, so a rule can be statically checked for reachability and the Rule Store can reject a rule that can never fire or that shadows an earlier one.
- Policies are versioned exactly like lambdas and AUIL components: a new `lambda.register` on a policy name creates an immutable new version; a syntactically or semantically invalid policy fails to load, and the Broker falls back to the last-known-good version for that decision surface rather than fail-open.

---

## 3. Decision model

Every request the Broker evaluates resolves to exactly one of four outcomes:

| Decision | Meaning | Who acts next |
|---|---|---|
| `ALLOW` | Request proceeds immediately | Requesting component (lambda spawns, patch applies, kernel op executes) |
| `DENY` | Request is rejected, with a structured reason | Returned to the Agent Core, which may retry with a narrower request or explain the limitation to the user — never silently retried by the Broker itself |
| `CONFIRM` | Request requires explicit, out-of-band human approval before proceeding | Confirmation Surface Daemon (§9) — never the requesting component's own UI |
| `HOLD` | Request is neither approved nor denied; it's queued pending anomaly review | Anomaly Detector (§6); resolves to `ALLOW`/`DENY`/`CONFIRM` after review or auto-times-out to `DENY` |

`CONFIRM` and `HOLD` are structurally distinct: `CONFIRM` is *expected* friction for a known-sensitive class of action (the policy author already decided this class always needs a human); `HOLD` is *unexpected* friction triggered by the Anomaly Detector noticing something the static policy didn't anticipate (a rate spike, an unusual capability co-occurrence). Both terminate in either a human decision or a timeout-to-deny — the Broker never blocks indefinitely.

---

## 4. Capability grant lifecycle

1. Requesting component (lambda manifest, kernel-op call, UI patch, intent registration) issues an MCP call to the Broker's `policy.check` method with the request body.
2. Decision Engine resolves the applicable policy document (§2), evaluates rules top-to-bottom.
3. On `ALLOW`: Broker returns a signed grant token, time-boxed and scoped to the exact request evaluated (not the general capability class) — the Process Supervisor (`lambda-server-spec.md` §3) and the UI Runtime both check for this token before acting, so a component that bypassed `policy.check` entirely simply can't get anything to execute.
4. On `DENY`: Broker returns the reason and the specific rule that fired. No token issued.
5. On `CONFIRM`: Broker hands off to the Confirmation Surface Daemon (§9) and blocks the caller (async, with a correlation id) until a decision or timeout.
6. On `HOLD`: Broker enqueues to the Anomaly Detector (§6) and blocks similarly.
7. **Grants are monotonic and non-escalating**, mirroring `lambda-server-spec.md` §2.2 exactly: a grant token is never broadened by anything other than a fresh `policy.check` call. A function or UI patch cannot "top up" an existing grant.

---

## 5. Schema-validated kernel & systemd operations

The Broker is the sole path between the Agent Core and the System Daemon (parent §3.1) and between the Agent Core and systemd (`agent-core-spec.md` §7). Both surfaces use the same shape:

- A **fixed, versioned operation schema** per allowed op (`power.set_profile`, `display.set_mode`, `net.set_interface_state`, `systemd.restart`, etc.) — free-form parameter payloads are rejected before they ever reach the Decision Engine; this is a parse-time gate, not a policy-rule gate, because "is this even a legal shape" shouldn't have to wait for rule evaluation.
- A **protected-unit list** (systemd units the Broker treats as load-bearing: the Broker itself, the Lambda Server, the State Store, the compositor, networking) — any `systemd.stop`/`restart`/`disable` targeting a protected unit is hard-wired to resolve to `CONFIRM`, not merely policy-configured to do so, so a malicious policy document can't quietly downgrade this to `ALLOW`. This hard-wire is the one exception to "everything is policy-configurable" in this spec, and it's deliberate.

---

## 6. Anomaly detection & rate limiting

- **Rate limiting** is per-capability-class and per-requesting-identity (a specific lambda name, or the Agent Core itself), using a sliding-window counter kept in the Broker's own local state (not the shared State Store, so a State-Store outage can't blind the Broker to a flood originating from the State Store's own client).
- **Anomaly triggers** (non-exhaustive, extend via policy):
  - Repeated `DENY` on the same capability class from the same requester in a short window (possible prompt-injection probing).
  - A capability combination absent from the requester's manifest history (a "weather widget" suddenly wanting filesystem write, per parent §3.3's own example).
  - Deploy velocity spike from the Lambda Server (many `lambda.register` calls in a short window — possible runaway synthesis loop).
- On trigger: the specific request moves to `HOLD` (§3); it does **not** retroactively revoke already-issued grants — revocation is a separate, always-available `policy.revoke` call the Broker can issue proactively if a held pattern escalates.

---

## 7. Audit log

- **Append-only, immutable, locally stored**, queryable via `policy.audit_query(filter)`.
- One entry per `policy.check` call: requester identity, method, full request body, decision, rule that fired, timestamp, and (if `CONFIRM`) the human's response and the confirmation surface's session id.
- Per `auil-asl-spec.md` §8: intent-registry registrations are logged **per-registration**, not per-call — "agent registered lambda X as handler for `calc.*` at time T" is the audit event; the high-frequency calls that follow are the registered lambda's own business and don't re-enter the Broker per call, only if the manifest itself changes.
- Motion events (`auil-asl-spec.md` §3.4) structurally never appear here, by construction — they never call the Broker in the first place.
- The log itself is a protected resource: reading it requires no special capability (it's meant to be inspectable), but *writing* to it is only ever done by the Decision Engine internally — there is no MCP method that lets any other component insert or edit an entry.

---

## 8. Prompt-injection containment

Per parent §3.3: content the agent reads from outside (web pages, files, subtitles) is data, never instruction. The Broker's enforcement mechanism, concretely:

- Every `policy.check` call from the Agent Core carries a **provenance tag**: which turn / user utterance the request traces back to, generated by the Agent Core's session loop, not self-reported free text.
- The Broker does not (and structurally cannot) verify the *semantic* truth of a provenance tag — that would require trusting the agent's own claims, which defeats the purpose. Instead, it enforces a **shape constraint**: a capability request whose provenance tag references a document/ingested-content id rather than a live user-utterance id is automatically downgraded to at least `CONFIRM`, regardless of what policy would otherwise say. Reading a webpage can never, by itself, produce an `ALLOW`-tier request.
- This is a blunt instrument by design — false positives (a legitimate task that happens to be triggered while reading a document) cost a confirmation click; false negatives (a genuinely injected instruction slipping through as `ALLOW`) are the failure mode worth over-correcting against.

---

## 9. The out-of-band confirmation surface

**The problem** (carried over from the parent architecture, unresolved until now): if `CONFIRM` decisions are rendered as an AUIL dialog, the Agent Core — or a lambda it deployed — is the thing drawing the "are you sure?" prompt the human is relying on to catch it. A compromised or simply wrong agent can render a dialog that says "Confirm: reboot" while actually requesting something else, or can render nothing and just proceed if the UI Runtime doesn't distinguish "no confirmation needed" from "confirmation was faked."

**The resolution:** confirmation is rendered by a component the Agent Core cannot write to, using content the Agent Core cannot compose.

1. **A reserved compositor surface**, analogous to a Wayland session-lock surface: the Wayland Compositor (parent §3.6.1) exposes exactly one protocol role — `confirmation-surface` — that only the Confirmation Surface Daemon (a process owned by the Broker, L2) may bind. No other client, including anything the Agent Core deploys, can acquire this role; the compositor enforces this at the protocol level, the same way session-lock surfaces are unfakeable by ordinary Wayland clients today.
2. **Fixed, non-LLM templates.** The Daemon does not render arbitrary AUIL/HTML. It fills named placeholders (`{action}`, `{target}`, `{requesting_capability}`, `{policy_rule}`) into a small, hand-authored, versioned set of templates shipped with the Broker itself. Any string interpolated into a placeholder is escaped as inert text — never as markup, never as something that can restyle or resize the surface. There is no code path by which agent-authored content becomes surface *structure*.
3. **Randomized affirmative control.** The actual "yes" control's position/label is randomized per confirmation instance from a small fixed set (not free text) and the correlation id is a short-lived nonce generated by the Broker. This defeats a class of attack where a compromised component tries to pre-position a fake click or synthetic input at a predictable coordinate — it doesn't defend against a human being tricked by content, since the content itself is Broker-authored and constrained by (2), not agent-authored.
4. **Input provenance.** The confirming input event must arrive through the System Daemon's raw input path (parent §3.1) tagged as physically-originated — the same path real keystrokes/clicks already take, with no software path from the Agent Core or any lambda able to synthesize an equivalent event. This is an existing property of the input stack (parent Design Commitment #2: real-time input never touches inference), reused here rather than invented new.
5. **Fail-closed timeout.** No response within a policy-configured window (default 60s) resolves to `DENY`, logged as `CONFIRM_TIMEOUT`.

This doesn't need new kernel work beyond what a modern Wayland compositor already supports for lock screens — it's a scoping decision (who's allowed to bind the role), not a new mechanism.

---

## 10. MCP control surface

| Tool | Purpose |
|---|---|
| `policy.check(method, request, provenance)` | The universal decision entrypoint (§3–§4). Returns `ALLOW` + grant token, `DENY` + reason, or a pending correlation id for `CONFIRM`/`HOLD`. |
| `policy.confirm_result(correlation_id)` | Poll/await the outcome of a `CONFIRM`/`HOLD` request. |
| `policy.revoke(grant_token)` | Proactively kill an already-issued grant (used by the Anomaly Detector, or manually). |
| `policy.register(policy_doc)` | Load a new/updated policy document version (§2). Loading itself goes through a bootstrap policy that only the system's initial setup — not the Agent Core — may satisfy for the *core* decision surfaces. |
| `policy.get_policy(name, version?)` | Inspect a policy document (auditability, debugging). |
| `policy.audit_query(filter)` | Query the audit log (§7). |
| `policy.list_grants(requester?)` | Introspect currently-live grant tokens. |

---

## 11. Cross-cutting validations

The same Decision Engine, different `applies_to` policy documents, is the enforcement point for every deferred validation named in the other specs:

- `component.definition` — enforces the depth-3 inheritance cap (`auil-asl-spec.md` §2.4).
- `ui.patch` — structural validation of patch ops before the UI Runtime applies them (`auil-asl-spec.md` §7), including slot-fill validation (`auil-asl-spec.md` §9.4).
- `mcp.intent-register` — validates an `exposes_mcp` manifest claim before adding an entry to the L3 intent registry (`auil-asl-spec.md` §8, `lambda-server-spec.md` §6).
- `lambda.capability-grant` — the CAPS power-set check (`lambda-server-spec.md` §2).

---

## 12. Security summary

| Threat | Mitigation |
|---|---|
| Agent fakes its own confirmation dialog | Confirmation rendered on a compositor-protected surface the agent cannot bind to, with Broker-authored, non-markup content (§9) |
| Policy document silently weakens a hard-wired protection | Protected-unit `CONFIRM` requirement is hard-wired in the Decision Engine, not policy-overridable (§5) |
| Injected instruction from ingested content escalates privilege | Provenance tag shape-check downgrades document-sourced requests to at least `CONFIRM` regardless of policy (§8) |
| Runaway capability requests / deploy floods | Per-identity rate limiting + anomaly `HOLD` state (§6) |
| Broker itself compromised | Broker is capability-scoped under itself at boot (bootstrap policy), sandboxed like any L1 component; audit log is append-only and not writable by any MCP method (§7) |
| Confirmation surface attacked via predictable synthetic input | Randomized control placement + input provenance requirement restricted to the physically-originated path (§9) |

---

## 13. Open items before implementation

1. **Bootstrap policy** — the policy documents governing `policy.register` itself need to exist before any agent runs; who authors v1 of these, and how are they distributed/signed as part of the OS image rather than generated at runtime?
2. **Compositor protocol extension** for the `confirmation-surface` role — needs to be specified precisely enough to submit as (or adapt from) a real Wayland protocol extension, not just described here.
3. **Cross-device confirmation** — if the machine has no display available (headless, or display subsystem down) when a `CONFIRM` fires, what's the fallback out-of-band channel? Needs a decision, not silent `DENY`-on-timeout as the only path.
4. **Policy rule static analysis tooling** — the "reject unreachable/shadowed rules" property claimed in §2 needs an actual checker, not just an assertion in this doc.
5. **Grant token format & signing** — HMAC vs. asymmetric, and where the signing key lives, given the Broker itself is a sandboxed process like everything else.
