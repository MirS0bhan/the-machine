# Testing & Coverage

## Running tests

```bash
make test-all            # Rust + Python + build-script tests (recommended)
make test-rust           # cargo test --workspace
make test-python         # integration + component tests
make test-build-scripts  # release assemble regression (CI rust-* glob)
make lint                # rustfmt + clippy (mcp-bus)
make verify              # tests + release build + docs + inventory + initramfs
make verify-docs         # documentation ↔ code cross-check only
```

## Python coverage

Integration tests in `tests/integration/` exercise the Python MCP reference servers in-process. Component suites:

| Package | Location | Focus |
|---------|----------|-------|
| lambda-server | `lambda-server/test_server.py` | registry, sandbox, MCP |
| policy-broker | `policy-broker/tests/` | rule interpreter |
| state-store | `state-store/tests/` | backends, pub/sub |
| local-model | `local-model/tests/` | privacy, embeddings |
| ui-engine | `ui-engine/test_engine.py` | AUIL, patches |
| ui-engine-demo | `ui-engine-demo/test_demo.py` | end-to-end terminal demo |

## Rust coverage

```bash
make coverage   # writes build/coverage.lcov (gitignored)
```

CI uploads the `coverage-lcov` artifact on every `main` push.
Do not name that artifact `rust-*`: `package-release` downloads every `rust-*`
artifact and must only treat same-named executables as service binaries.

### Unit tests (Rust)

| Crate | Module | Tests |
|-------|--------|-------|
| policy-broker | `policy_engine.rs` | allow/deny/first-match/default-deny |
| mcp-bus | `registry.rs` | exact/wildcard resolve, pattern matching, deregister |
| mcp-bus | `auth.rs` | `_bus.register` identity / namespace / handler checks |
| mcp-bus | `main.rs` | newline-delimited responses, prefix fallback |
| compositor | `pixel.rs` | framebuffer buffer, color hash |
| system-daemon | `input.rs` | evdev struct size |
| lambda-server | `validate.rs` | forbidden patterns, schema inference |
| agent-core | `main.rs` | `agent.status`, `agent.cloud.status`, local_only_mode guards |
| agent-core | `planner.rs` | heartbeat + calculator plans |
| local-model-daemon | `engine.rs` | classify stub, embed determinism |

Most daemon `main.rs` handlers are covered indirectly via Python integration tests against the HTTP reference servers and boot-path manual verification. Expanding Rust socket-level integration tests is tracked as future work.

### Coverage expectations

- **Critical paths** (policy engine, registry, lambda validation, planner heuristics) must keep unit tests green.
- **CI** runs `cargo llvm-cov --workspace --summary-only` and archives LCOV; no hard percentage gate yet (overall ~7% line coverage — most logic lives in binary mains).
- Run `make verify-docs` after changing `docs/reference/component-inventory.yaml` or boot service lists.

## Documentation verification

`scripts/verify-docs-code.py` checks:

1. `component-inventory.yaml` ↔ `mkinitramfs.sh` ↔ `verify-all.sh` ↔ `Cargo.toml`
2. Key docs mention all boot services
3. MCP methods in inventory exist in Rust sources
4. Environment variables documented in guides
5. No stale phrases in `runtime-model.md`
6. No unresolved git conflict markers in `docs/` or crate `src/**/*.rs`

Wired into `make verify` and the CI `test` job.
