# Lambda Execution Server

> **Overlap warning:** This directory contains **two implementations** (Python + Rust).
> See [Python ↔ Rust Overlap Guide](../docs/guides/python-rust-overlap.md).
>
> | Workflow | Use |
> |----------|-----|
> | Tests, HTTP API, agent dev | **Python** (`*.py`) |
> | ISO boot, seccomp sandbox | **Rust** (`src/`) |

A production-ready implementation of the Lambda Execution Server as specified in `docs/spec.md`.

[![Python 3.10+](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

The Lambda Execution Server provides a secure, isolated environment for executing functions with capability-based access control. It implements:

- **Function Registry**: Named, described, persistent, reusable functions with version history
- **Process Supervisor**: Sandboxed function processes with warm pool management
- **IPC Router**: Inter-function calls with capability enforcement and fast-path leases
- **MCP Control Interface**: Tools for agent interaction (search, register, invoke, etc.)

## Architecture

```
┌───────────────────────────────────────────────────────────────┐
│  Lambda Execution Server                                       │
│                                                                │
│   ┌───────────────┐  ┌────────────────┐  ┌──────────────────┐ │
│   │ Function       │  │ Process         │  │ IPC Router /     │ │
│   │ Registry       │  │ Supervisor      │  │ Capability       │ │
│   │ (name, desc,   │  │ (spawn/kill,    │  │ Enforcer         │ │
│   │  schema, caps, │  │  warm pools,    │  │ (resolve target, │ │
│   │  version hist) │  │  cgroups)       │  │  check CAP_IPC,  │ │
│   └───────┬────────┘  └────────┬────────┘  │  issue leases)   │ │
│           │                    │             └────────┬────────┘ │
│   ┌───────▼────────────────────▼───────────────────────▼───────┐│
│   │           Per-function sandboxed process pool               ││
│   │   [x: python] ◄──IPC socket──► [y: python] ◄──► [z: go]   ││
│   └────────────────────────────────────────────────────────────┘│
│                                                                │
│   ┌──────────────────────────────────────────────────────────┐ │
│   │  MCP Control Interface (lambda.search / .register / ...)  │ │
│   └──────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────┘
```

## Quick Start

### Using Docker (Recommended)

```bash
# Build and run
docker-compose up lambda-server

# Or for development with hot-reload
docker-compose up lambda-server-dev
```

### Local Development

```bash
# Install dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Run the server
python -m http_server
```

### Using the SDK

```python
from server import create_server

# Create server instance
server = create_server()

# Search for functions
result = server.handle_mcp_tool("lambda.search", {"query": "calculate"})

# Register a function
result = server.handle_mcp_tool("lambda.register", {
    "name": "calc.add",
    "runtime": "python3.12",
    "code": "def add(input): return {'sum': sum(input['values'])}",
    "description": "Adds two or more numeric values",
    "input_schema": {"values": "number[]"},
    "output_schema": {"sum": "number"},
    "capabilities": "pure",
})

# Invoke a function
result = server.handle_mcp_tool("lambda.invoke", {
    "name": "calc.add",
    "input": {"values": [1, 2, 3]},
})
```

## MCP Tools

| Tool | Purpose |
|---|---|
| `lambda.search(query)` | Semantic/keyword search over registry |
| `lambda.describe(name)` | Full manifest for one function |
| `lambda.register(...)` | Create or update a function |
| `lambda.invoke(name, input)` | Direct invocation |
| `lambda.deprecate(name, version)` | Mark version as deprecated |
| `lambda.rollback(name, version)` | Rollback to previous version |
| `lambda.list_calls(name)` | Introspect IPC call graph |
| `lambda.list_functions()` | List all registered functions |
| `lambda.list_processes()` | List running processes |
| `lambda.list_warm_pool()` | List warm pool entries |
| `lambda.get_call_log()` | Get IPC call log |
| `lambda.get_stats()` | Get server statistics |

## HTTP API

The server exposes an HTTP API for external access:

```bash
# Health check
curl http://localhost:8080/health

# List tools
curl http://localhost:8080/tools

# Search for functions
curl -X POST http://localhost:8080/mcp/lambda.search \
  -H "Content-Type: application/json" \
  -d '{"query": "calculate"}'

# Register a function
curl -X POST http://localhost:8080/mcp/lambda.register \
  -H "Content-Type: application/json" \
  -d '{
    "name": "calc.add",
    "runtime": "python3.12",
    "code": "def add(input): return {\"sum\": sum(input[\"values\"])}",
    "description": "Adds two or more numeric values",
    "input_schema": {"values": "number[]"},
    "output_schema": {"sum": "number"},
    "capabilities": "pure"
  }'
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `LAMBDA_HOST` | `0.0.0.0` | Host to bind to |
| `LAMBDA_PORT` | `8080` | Port to listen on |
| `LAMBDA_DEBUG` | `false` | Enable debug mode |
| `LAMBDA_LOG_LEVEL` | `INFO` | Logging level |
| `LAMBDA_SOCKET_DIR` | `/tmp/lambda-sockets` | Directory for IPC sockets |
| `LAMBDA_MAX_WARM_PER_FUNCTION` | `2` | Maximum warm processes per function |
| `LAMBDA_WARM_TIMEOUT_SECONDS` | `300` | Warm process timeout |
| `LAMBDA_HEARTBEAT_INTERVAL` | `30` | Heartbeat interval |
| `LAMBDA_MAX_TOTAL_WARM` | `50` | Maximum total warm processes |
| `LAMBDA_LEASE_TTL_SECONDS` | `300` | Lease TTL |

### Configuration File

Create a `.env` file for Docker Compose:

```env
LAMBDA_PORT=8080
LAMBDA_LOG_LEVEL=INFO
LAMBDA_MAX_WARM_PER_FUNCTION=4
LAMBDA_WARM_TIMEOUT_SECONDS=600
```

## Capability Model

Capabilities are a closed, versioned enum:

```python
from models import Capability

# Available capabilities
Capability.NET_OUT      # Outbound network
Capability.NET_IN       # Listen for inbound
Capability.FS_READ      # Read filesystem
Capability.FS_WRITE     # Write filesystem
Capability.STATE_READ   # Read state store
Capability.STATE_WRITE  # Write state store
Capability.IPC_CALL     # Call other functions
Capability.GPU          # GPU access
# ... and more
```

### Presets

Presets expand to capability subsets:

- `pure`: No capabilities (math, string processing)
- `reader`: STATE_READ + FS_READ
- `networked`: NET_OUT

```python
# Use preset
server.handle_mcp_tool("lambda.register", {
    "name": "calc.add",
    "capabilities": "pure",  # Expands to empty set
    # ...
})

# Or explicit capabilities
server.handle_mcp_tool("lambda.register", {
    "name": "api.client",
    "capabilities": [
        {"capability": "NET_OUT", "domains": ["api.example.com"]}
    ],
    # ...
})
```

## Python SDK

Functions use the SDK to make IPC calls:

```python
from lambda_sdk import call, state, capabilities

@capabilities(ipc_call=["y"])
def x(input):
    output = call("y", input)  # Looks synchronous; is IPC under the hood
    return transform(output)

# State access
@capabilities(state_read=["myapp/config"])
def read_config(input):
    config = state.get("myapp/config")
    return config
```

## Development

### Running Tests

```bash
# Run all tests
pytest

# Run with coverage
pytest --cov=lambda_server

# Run specific test file
pytest tests/test_registry.py
```

### Type Checking

```bash
# Run mypy
mypy lambda_server/

# Run with strict mode
mypy --strict lambda_server/
```

### Linting

```bash
# Run ruff
ruff check lambda_server/

# Run with auto-fix
ruff check --fix lambda_server/
```

### Docker Development

```bash
# Build for development
docker-compose build lambda-server-dev

# Run with hot-reload
docker-compose up lambda-server-dev

# View logs
docker-compose logs -f lambda-server-dev
```

## Production Deployment

### Docker Compose (Simple)

```bash
# Set environment variables
export LAMBDA_PORT=8080
export LAMBDA_LOG_LEVEL=INFO
export LAMBDA_MAX_WARM_PER_FUNCTION=4

# Deploy
docker-compose up -d lambda-server
```

### With Monitoring

```bash
# Deploy with Prometheus and Grafana
docker-compose --profile monitoring up -d

# Access:
# - Lambda Server: http://localhost:8080
# - Prometheus: http://localhost:9090
# - Grafana: http://localhost:3000
```

### Kubernetes

See `k8s/` directory for Kubernetes manifests (if applicable).

## Security

### Capability-Based Access Control

- Functions only get capabilities they declare
- IPC calls are validated against declared call graph
- No function can access resources it's not granted

### Process Isolation

- Each function runs in its own process
- seccomp filters restrict system calls
- Network and mount namespaces isolate resources
- cgroups limit CPU and memory usage

### Audit Logging

All IPC calls are logged for security auditing:

```python
result = server.handle_mcp_tool("lambda.get_call_log", {
    "caller": "orchestrator",
    "limit": 100,
})
```

## Project Structure

```
lambda-server/
├── docs/
│   └── spec.md              # Original specification
├── lambda_server/
│   ├── __init__.py          # Package initialization
│   ├── models.py            # Core data models
│   ├── registry.py          # Function registry
│   ├── enforcer.py          # Capability enforcement
│   ├── supervisor.py        # Process management
│   ├── router.py            # IPC routing
│   ├── mcp_interface.py     # MCP control interface
│   ├── server.py            # Main server class
│   ├── sdk.py               # Python SDK
│   ├── config.py            # Configuration management
│   └── http_server.py       # HTTP API server
├── tests/
│   ├── __init__.py
│   └── test_server.py       # Test suite
├── Dockerfile               # Docker build
├── docker-compose.yml       # Docker Compose config
├── pyproject.toml           # Python project config
├── .dockerignore            # Docker ignore rules
├── .gitignore               # Git ignore rules
└── README.md                # This file
```

## Open Items (from spec)

1. **Wire format** for IPC layer (msgpack recommended)
2. **`lambda.search` ranking** (embedding-based semantic search)
3. **Resource quotas per capability tier**
4. **Capability power-set versioning**
5. **Cross-function schema evolution**
6. **WASM tier relationship to native sandbox**

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Specification: `docs/spec.md`
- Architecture: `agent-native-os-architecture.md`
- MCP Protocol: `auil-asl-spec.md`
