# L1State Store Specification

## Purpose and Scope
The L1State Store is the single source of truth for the system's state. It provides persistent, consistent, and concurrent access to state data for all components.

## Key Responsibilities
- State persistence (disk/in-memory hybrid)
- Concurrency control (optimistic/pessimistic locking)
- State query and update APIs
- Event sourcing for state changes

## Dependencies
- **L1Event Bus**: For publishing state change events
- **L2Policy Broker**: For policy-aware state updates
- **L4Agent Core**: For agent-initiated state queries

## Interfaces
- **MCP Tools**: `state_get`, `state_update`, `state_query`
- **Events**: Publishes `state_updated`
- **CLI**: `state-cli` for state inspection

## Data Models
```python
class State:
    key: str
    value: Any
    version: int
    last_updated: datetime
```

## Open Questions
- What are the consistency requirements for distributed state?
- Should state be sharded or partitioned?
- How to handle state migration?