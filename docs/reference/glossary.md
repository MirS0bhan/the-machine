# Glossary

| Term | Definition |
|------|------------|
| **Agent Core** | L4 decision harness — session loop, hybrid LLM router, MCP client |
| **AUIL** | Agent UI Layout — line-oriented declarative UI structure language |
| **ASL** | Agent State Language — patch/diff format for UI updates |
| **Broker** | Policy Broker (L2) — capability enforcement, audit, confirmation |
| **CAP_*** | Capability tokens declared in lambda manifests and checked by the Broker |
| **Event Bus** | L1 reactive router — decides when to wake the Agent Core |
| **Fallback Shell** | L5 recovery UI that works with zero inference |
| **Lambda** | Sandboxed function deployed on the Lambda Execution Server |
| **MCP** | Model Context Protocol — the system's uniform IPC/audit bus |
| **MCP Bus** | L3 message fabric — routes MCP calls between components |
| **State Store** | L1 persistent store for UI State Tree and system/task state |
| **UI State Tree** | Declarative document the UI Runtime renders |
| **Warm pool** | Pre-spawned lambda processes for low-latency invocation |
