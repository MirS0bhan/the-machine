"""
Lambda Execution Server

Fills: §3.2.1 of agent-native-os-architecture.md (Lambda Execution Server)
and §7.3 ("Lambda base images")

Architecture:
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

Version: 0.1.0
"""

from server import LambdaServer, create_server
from models import (
    Capability,
    CapabilityGrant,
    CapabilityPreset,
    FunctionManifest,
    FunctionVersion,
    IPCLease,
    ProcessHandle,
)
from registry import FunctionRegistry
from enforcer import CapabilityEnforcer
from supervisor import ProcessSupervisor
from router import IPCRouter
from mcp_interface import MCPControlInterface

__version__ = "0.1.0"
__author__ = "Lambda Server Team"

__all__ = [
    # Main server
    "LambdaServer",
    "create_server",
    
    # Models
    "Capability",
    "CapabilityGrant",
    "CapabilityPreset",
    "FunctionManifest",
    "FunctionVersion",
    "IPCLease",
    "ProcessHandle",
    
    # Components
    "FunctionRegistry",
    "CapabilityEnforcer",
    "ProcessSupervisor",
    "IPCRouter",
    "MCPControlInterface",
]
