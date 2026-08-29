# Lambda Execution Server — Function Registry, Process Isolation, IPC & MCP Control
 
**Fills:** §3.2.1 of `agent-native-os-architecture.md` (Lambda Execution Server) and §7.3 ("Lambda base images")
**Related:** `auil-asl-spec.md` §8 (MCP as a routing fabric) — this document is the server that §8 registers handlers *into*
**Version:** 0.1 (design draft)
**Status:** Conceptual, pre-implementation
 
---
 
## 0. Design goals
 
1. **Functions are named, described, persistent, reusable.** The agent's job is to make a capability exist once, not to regenerate code every time a similar request comes in.
2. **Process is the trust boundary.** One function = one sandboxed process (or one warm-pool slot). Capability grants are attached to processes, not to code — code alone proves nothing to the sandbox.
3. **Cross-function calls are IPC, always** — never an in-process import, even for two functions in the same language sitting in the same warm pool. This is what makes the call graph inspectable and the capability model enforceable: if `x` calls `y`, that edge exists somewhere the Broker can see and gate, not buried in a language-level `import y`.
4. **Capabilities are a closed, versioned power set**, not free-form strings — same philosophy the parent doc applies to kernel operations (§3.3) and this doc applies one level down, to inter-function calls.
5. **The SDK is the only door.** A function's code never touches a raw socket. It calls `call("y", input)`; the framework decides whether that's a brokered round-trip or a leased fast-path channel, and refuses the call outright if the manifest didn't declare it.
6. **The server exposes itself over MCP**, so the agent's whole relationship to "write some code" becomes: search first, register once, never write it again.
 
---
 
## 1. Component map (expanding parent doc §3.2.1)
 
```
┌───────────────────────────────────────────────────────────────┐
│  Lambda Execution Server (L1)                                  │
│                                                                  │
│   ┌───────────────┐  ┌────────────────┐  ┌──────────────────┐  │
│   │ Function       │  │ Process         │  │ IPC Router /     │  │
│   │ Registry       │  │ Supervisor      │  │ Capability       │  │
│   │ (name, desc,   │  │ (spawn/kill,    │  │ Enforcer         │  │
│   │  schema, caps, │  │  warm pools,    │  │ (resolve target, │  │
│   │  version hist) │  │  cgroups)       │  │  check CAP_IPC,  │  │
│   └───────┬────────┘  └────────┬────────┘  │  issue leases)   │  │
│           │                    │             └────────┬────────┘ │
│           │                    │                       │          │
│   ┌───────▼────────────────────▼───────────────────────▼───────┐ │
│   │           Per-function sandboxed process pool                │ │
│   │   [x: python] ◄──IPC socket──► [y: python] ◄──► [z: go]      │ │
│   └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│   ┌──────────────────────────────────────────────────────────┐ │
│   │  MCP Control Interface (lambda.search / .register / ...)  │ │
│   └──────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────┘
                All of the above is itself one container
                (or one microVM), sitting under the L2 Broker,
                same as every other L1 component in the parent doc.
```
 
Every arrow that crosses a process boundary in this diagram is IPC. Every capability grant that lets an arrow exist was checked by the Broker before the process was ever spawned.
 
---
 
## 2. Capability model — the CAPS power set
 
### 2.1 The fixed set
 
Capabilities are a closed, versioned enum — a function's manifest declares a **subset** of this set (an element of its power set), never a free-form string. This mirrors the parent doc's stance on kernel operations: no capability the Broker doesn't already know how to validate.
 
```
CAP_NET_OUT(domains=[...])         — outbound network, scoped to named domains
CAP_NET_IN(port)                   — listen for inbound connections (rare; most functions don't need this)
CAP_FS_READ(paths=[...])
CAP_FS_WRITE(paths=[...])
CAP_MIC / CAP_CAMERA
CAP_GPU(scope=render|compute)
CAP_STATE_READ(paths=[...])        — State Store (§3.2.2 parent doc) read
CAP_STATE_WRITE(paths=[...])
CAP_IPC_CALL(targets=[name, ...])  — which OTHER functions this one may call
CAP_SPAWN_EPHEMERAL                — may ask the Supervisor for a throwaway sub-process
CAP_TIMER                          — may schedule itself via the Event/Scheduler Bus
CAP_SYS_PARAM(scope=[...])         — narrow, pre-approved sysctl-equivalents (rare; mirrors parent §3.3)
```
 
`CAP_IPC_CALL` is the one that matters most for this spec: it's a **declared call graph edge**, not a blanket "can do IPC" flag. If `x`'s manifest lists `CAP_IPC_CALL(targets=[y])` and `x`'s code tries to call `z`, the Enforcer rejects it before a socket is even opened — regardless of what `z`'s own permissions are.
 
### 2.2 Grant is monotonic, non-escalating
 
A function can never be granted more than its manifest declares, and the manifest can never expand without going back through the Broker (same as any other capability grant in the parent doc, §3.3). Two enforcement layers, deliberately redundant:
 
- **Kernel-level:** the Process Supervisor derives an actual seccomp filter + network namespace + mount namespace from the granted subset at spawn time. This is the layer that can't be lied to by buggy or malicious code.
- **SDK-level:** the per-language framework (§4) refuses to even attempt a `call()` to an undeclared target and raises immediately, so well-behaved code fails fast and loud instead of hitting a kernel wall silently.
 
### 2.3 Capability tiers as a shorthand (optional, for the Broker's UX)
 
To avoid the agent having to hand-author a capability list for every trivial function, the Registry can offer named presets that just expand to a fixed subset — `pure` (no caps at all — math, string processing, data transforms), `reader` (`CAP_STATE_READ` + `CAP_FS_READ` on a scoped path), `networked` (adds `CAP_NET_OUT` to a declared domain list). Presets are sugar; the Broker still validates the expanded set, not the preset name.
 
---
 
## 3. Process & warm pool model
 
- **One process per function**, isolated the same way the parent doc isolates lambdas generally (§3.2.1): OCI container or microVM, seccomp + namespaces, cgroup resource limits.
- **Warm vs cold**, same rule as the parent doc: frequently-hit or latency-sensitive functions (a calculator, a UI-bound handler) stay warm; rare one-shot functions cold-start per call.
- **Compiled languages pay their cost once.** Go/Rust functions are built in an ephemeral builder container on first `lambda.register`, and the compiled artifact — not the source — is what the Supervisor spawns from then on. Interpreted languages (Python/Ruby/R/JS) skip this step; the agent's iteration loop is faster for those, which is part of why they're the default for agent-authored glue.
 
---
 
## 4. IPC & the per-language SDK
 
### 4.1 The call, from the agent's/function's point of view
 
Given the scenario in the prompt — function `x` calls `y`, gets `output = lambda_server(y, input)` — the SDK makes that look like a normal function call, but every call is actually IPC underneath:
 
```python
# Python SDK
from lambda_sdk import call, state, capabilities
 
@capabilities(ipc_call=["y"])
def x(input):
    output = call("y", input)      # looks synchronous; is IPC under the hood
    return transform(output)
```
 
```javascript
// Node SDK
const { call, state, capabilities } = require("lambda-sdk");
 
capabilities({ ipcCall: ["y"] });
 
async function x(input) {
  const output = await call("y", input);   // Promise-wrapped IPC round trip
  return transform(output);
}
```
 
The same shape holds for Ruby (`LambdaSDK.call("y", input)`), R (`lambda_call("y", input)`), and Go/Rust (`sdk.Call(ctx, "y", input)` — typed, since compiled languages get compile-time schema checks against the target's declared `input_schema`/`output_schema` for free).
 
**Cross-language calls are normal.** Since the wire format is schema-typed bytes over a socket, not language-native objects, a JS function can call a Python function can call a Go function without either side knowing what the other is written in. The Registry's `input_schema`/`output_schema` (§6) is the actual contract; the language is an implementation detail of the callee.
 
### 4.2 Two call paths, one API
 
The SDK's `call()` doesn't force the caller to know or care which path is used — that's resolved underneath:
 
| Path | When | Mechanism |
|---|---|---|
| **Brokered call** | First call to a target in this process's lifetime, or target isn't warm | `call()` → IPC Router: checks `CAP_IPC_CALL` grant, resolves/spawns `y`, proxies input, returns output. Every brokered call is logged. |
| **Fast-path lease** | Repeat calls to the same target (e.g. a tight loop, or a UI-bound handler called every frame-adjacent tick) | Router hands back a **TTL-bound, capability-scoped socket lease** on first resolution. Subsequent `call()`s use the leased socket directly, process-to-process, no Router round-trip. Lease auto-expires and re-brokers periodically; Router can revoke it immediately if the manifest's grant changes. |
 
This gets you both things asked for: `output = lambda_server(y, input)` semantics for the common/first case, and direct IPC for the hot-loop case — without ever letting a function open an arbitrary socket itself. The lease is still something the Router issued and can kill; it's a fast lane, not a bypass.
 
### 4.3 State access
 
`state.get(path)` / `state.set(path, value)` in every SDK map to the parent doc's State Store (§3.2.2), gated by `CAP_STATE_READ`/`CAP_STATE_WRITE`. Same pattern as IPC: looks like a local call, is actually a scoped, capability-checked round trip.
 
---
 
## 5. Container & language toolchain
 
Rather than one bloated image with every interpreter installed, the container is **layered**: a minimal base (Supervisor, Router, Registry client, seccomp profiles) plus **per-language runtime layers** pulled in only when a function actually declares that runtime. Keeps cold-start images small and attack surface proportional to what's actually deployed.
 
| Language | Why it's in the default set |
|---|---|
| **Python** | Requested; also the deepest ecosystem for data/glue/ML work, and the language most agent-generated one-off functions will land in |
| **JavaScript/TypeScript (Node)** | Requested; natural fit given the rest of the OS already speaks JSON/MCP-shaped idioms, and it's the ecosystem most web-scraping/HTTP-glue code assumes |
| **Ruby** | Requested; strong for text/scripting-style glue, still the default in some domains (Rails-adjacent APIs, certain CLIs the agent might need to shell out to) |
| **R** | Requested; the natural backend for the `chart` AUIL primitive and any statistics-heavy function |
| **Go** *(suggested addition)* | Compiled, small static binaries, fast cold-start relative to a JVM-style runtime — good for latency-sensitive functions the agent promotes out of Python once they're proven hot (e.g. the IPC Router itself could be Go) |
| **Rust** *(suggested addition)* | For the rare function that needs to be a genuinely vetted, memory-safe primitive — the parent doc already says the agent shouldn't hand-roll crypto/decoders; Rust is the language those *vetted* base-image primitives are written in, and it's available if a function needs to link against one directly |
| **WASM runtime (wasmtime)** *(suggested addition)* | An extra containment layer, independent of source language: agent-generated code that's lower-trust (first-run, unreviewed, or from a less-trusted synthesis path) can be compiled to WASM and run under WASI's own capability model as a second sandbox inside the process sandbox — defense in depth for exactly the code the agent is least sure about |
| **Bash/POSIX shell** *(suggested addition, tightly capped)* | Thin orchestration/glue only — chaining existing vetted CLI tools (ffmpeg, etc., per parent doc §3.2.1). Should almost never be granted `CAP_FS_WRITE` or `CAP_NET_OUT` beyond what it's explicitly gluing together; treat as the lowest-trust-by-default runtime in the menu |
 
---
 
## 6. Function Registry — entry schema
 
```
function calc.add
  version: 3
  runtime: python3.12
  description: "Adds two or more numeric values, returns their sum."
  input_schema:  { values: number[] }
  output_schema: { sum: number }
  capabilities:  pure                         # expands to: (none)
  exposes_mcp:   calc.add                     # optional — registers as a direct MCP handler
                                                # per auil-asl-spec.md §8
  source: registry://calc/add/v3/main.py
  artifact: none                              # interpreted; no build step
  status: warm
  history: [v1 (2026-03-01), v2 (2026-04-11), v3 (2026-06-02, current)]
```
 
Compiled-language entries additionally carry `artifact:` pointing at the built binary and a `build_log:` reference. `exposes_mcp` is what lets a UI button's `on:press=mcp:calc.add` (AUIL) route straight to this function once it's registered, without the agent being invoked again — the mechanism defined in `auil-asl-spec.md` §8.
 
**Rollback** works exactly like the parent doc's lambda versioning generally (§3.2.1): every `lambda.register` on an existing name creates a new immutable version; the Supervisor auto-rolls-back to last-known-good on crash-loop or failed health check, same as any other lambda.
 
---
 
## 7. MCP control surface
 
This is the interface the Agent Core actually talks to — the Lambda Server is, from the Bus's point of view, just another MCP server:
 
| Tool | Purpose |
|---|---|
| `lambda.search(query)` | Semantic/keyword search over registry descriptions. Returns candidate `{name, description, input_schema, output_schema}` — this is the "is there already a function for this" step. |
| `lambda.describe(name)` | Full manifest for one function, including capability list and version history. |
| `lambda.register(name, runtime, code, description, input_schema, output_schema, capabilities, exposes_mcp?)` | Create or update a function. Triggers Broker capability validation → build (if compiled) → sandbox profile derivation → Registry entry. This is the "inject the function" step. |
| `lambda.invoke(name, input)` | Direct invocation — used when the agent (or another MCP client) wants a result immediately rather than through a UI-bound intent. |
| `lambda.deprecate(name, version)` / `lambda.rollback(name, version)` | Version lifecycle, mirrors parent doc §3.2.1. |
| `lambda.list_calls(name)` | Introspect a function's declared `CAP_IPC_CALL` graph — lets a human auditor or the Broker answer "what can this thing talk to" without reading the code. |
 
---
 
## 8. The workflow the prompt describes, end to end
 
> "Calculate something" →
 
1. Agent calls `lambda.search("calculate 47 * 12.5 with a running total")`.
2. **Hit:** a `calc.*` family already exists → `lambda.invoke("calc.eval", {...})` → done. No code written, no new process spawned if `calc.eval` is warm.
3. **Miss:** nothing matches → the agent, informed by a skill describing this Lambda Server's SDK conventions (§4), writes the function body in whichever runtime fits (Python, for a first-cut calculator) and its capability manifest (almost certainly `pure` — a calculator needs no network, filesystem, or state access).
4. Agent calls `lambda.register("calc.eval", "python3.12", <code>, <description>, <schemas>, capabilities="pure", exposes_mcp="calc.*")`.
5. Broker validates the manifest is a legal subset of the CAPS power set, Supervisor spawns the sandboxed process (or queues it warm), Registry stores it with the description that made it findable in step 1.
6. From here on: chat-driven calls hit it via `lambda.search` → `lambda.invoke` in one round trip with no code synthesis; UI-driven calls (a calculator button in AUIL) hit it directly via the `exposes_mcp` binding described in `auil-asl-spec.md` §8, without even the search step. Either way, the agent's inference is spent once, at creation time.
 
---
 
## 9. Security summary
 
| Threat | Mitigation |
|---|---|
| Function requests more than it needs | Manifest capabilities are a closed power set; Broker rejects anything outside the known enum, same posture as kernel ops in the parent doc |
| Function tries to call an undeclared target | `CAP_IPC_CALL(targets=[...])` is a declared call-graph edge; Enforcer rejects the call before a socket opens, independent of the callee's own permissions |
| Buggy/malicious agent-generated code | Process-per-function isolation (seccomp + namespaces + cgroups); crash-loop triggers automatic rollback to last-known-good version, same as parent doc §3.2.1 |
| Leaked or stolen fast-path lease | Leases are TTL-bound and capability-scoped; Router can revoke on manifest change; leases aren't raw unrestricted sockets, they're pre-authorized channels to one specific target |
| Supply-chain risk in language ecosystems (pip/npm/gem) | Vetted, versioned base images only; no arbitrary package installation at function runtime — same "glue, not reinvention" stance as the parent doc |
| Registry search results used as instructions rather than data | Search returns ranked metadata for the agent to *choose from*, never auto-invoked; `lambda.register` is the only path that creates a callable entry, and it goes through the same Broker validation as any other capability grant — an attacker can't get code executed just by getting a maliciously-described entry into search results, because search doesn't invoke |
| Compiled-artifact tampering between build and run | Registry stores an artifact hash alongside the binary path; Supervisor verifies hash at spawn |
 
---
 
## 10. Open items before implementation
 
1. **Wire format** for the IPC layer (length-prefixed msgpack is the likely default — cheap to parse in every target language, avoids JSON's per-call parsing overhead which matters at IPC volume even though it doesn't matter much at AUIL-authoring volume).
2. **`lambda.search` ranking** — pure keyword match is cheap but weak; embedding-based semantic search is better but adds a dependency the Lambda Server itself would have to run as... a lambda, which is a fun bootstrapping problem worth designing deliberately rather than accidentally.
3. **Resource quotas per capability tier** — CPU/memory/wall-clock limits should probably scale with what a function is trusted to do, not be flat across `pure` and `networked` functions alike.
4. **Capability power-set versioning** — how a new `CAP_*` gets added without invalidating every existing manifest that predates it (mirrors component-registry versioning in `auil-asl-spec.md` §9.3).
5. **Cross-function schema evolution** — if `calc.add`'s `output_schema` changes in v4, what happens to callers still declaring `CAP_IPC_CALL(targets=[calc.add])` against the v3 contract? Needs a compatibility policy, not just a version bump.
6. **WASM tier's relationship to the native sandbox** — is it a mode every function can opt into, or reserved specifically for lower-trust/first-run agent-generated code as suggested in §5? Worth deciding explicitly rather than letting it drift.
 
---
 
*End of document.*

