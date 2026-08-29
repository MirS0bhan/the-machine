# L2Policy Broker Specification

## Purpose and Scope
The L2Policy Broker is responsible for evaluating and enforcing policies that govern the behavior of the system. It acts as a gatekeeper for actions initiated by agents or external systems, ensuring compliance with predefined rules.

## Key Responsibilities
- Policy evaluation and enforcement
- Integration with L1State Store for context-aware decisions
- Event-driven policy triggers via L1Event Bus
- MCP tool surface for policy management

## Dependencies
- **L1State Store**: For state context during policy evaluation
- **L1Event Bus**: For event-driven policy triggers
- **L4Agent Core**: For agent-initiated policy checks

## Interfaces
- **MCP Tools**: `policy_evaluate`, `policy_manage`
- **Events**: Subscribes to `state_updated`, publishes `policy_decision`
- **CLI**: `policy-cli` for policy management

## Example Workflow
1. Agent requests action via MCP
2. Policy Broker evaluates request against current state
3. Broker publishes `policy_decision` event
4. System enforces decision

## Open Questions
- Should policies be versioned?
- How to handle policy conflicts?
- What are the performance requirements for policy evaluation?