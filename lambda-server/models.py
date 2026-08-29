"""
Core data models for the Lambda Execution Server.

This module defines the fundamental data structures used throughout the
Lambda Execution Server, including capability grants, function manifests,
process handles, and IPC leases.

Architecture Reference:
    - §3.2.1 of agent-native-os-architecture.md (Lambda Execution Server)
    - §7.3 ("Lambda base images")

Version: 0.1.0
"""

from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Any, Dict, List, Optional, Set
import hashlib
import json
import logging
import time
import uuid

logger = logging.getLogger(__name__)


class Capability(Enum):
    """
    Closed, versioned set of capabilities (CAPS power set).
    
    Capabilities are a closed, versioned enum — a function's manifest declares a
    **subset** of this set (an element of its power set), never a free-form string.
    This mirrors the parent doc's stance on kernel operations: no capability the
    Broker doesn't already know how to validate.
    
    Attributes:
        NET_OUT: Outbound network access, scoped to named domains
        NET_IN: Listen for inbound connections (rare; most functions don't need this)
        FS_READ: Read access to specified filesystem paths
        FS_WRITE: Write access to specified filesystem paths
        MIC: Microphone access
        CAMERA: Camera access
        GPU: GPU access (render or compute)
        STATE_READ: Read access to State Store paths
        STATE_WRITE: Write access to State Store paths
        IPC_CALL: Call other functions via IPC (declared call-graph edge)
        SPAWN_EPHEMERAL: May ask the Supervisor for a throwaway sub-process
        TIMER: May schedule itself via the Event/Scheduler Bus
        SYS_PARAM: Narrow, pre-approved sysctl-equivalents (rare)
    """
    NET_OUT = auto()
    NET_IN = auto()
    FS_READ = auto()
    FS_WRITE = auto()
    MIC = auto()
    CAMERA = auto()
    GPU = auto()
    STATE_READ = auto()
    STATE_WRITE = auto()
    IPC_CALL = auto()
    SPAWN_EPHEMERAL = auto()
    TIMER = auto()
    SYS_PARAM = auto()


class CapabilityPreset(Enum):
    """
    Named presets that expand to fixed capability subsets.
    
    Presets are sugar; the Broker still validates the expanded set, not the
    preset name. This avoids the agent having to hand-author a capability
    list for every trivial function.
    
    Attributes:
        PURE: No capabilities at all (math, string processing, data transforms)
        READER: STATE_READ + FS_READ on a scoped path
        NETWORKED: Adds NET_OUT to a declared domain list
    """
    PURE = set()
    READER = {Capability.STATE_READ, Capability.FS_READ}
    NETWORKED = {Capability.NET_OUT}


@dataclass(frozen=True)
class CapabilityGrant:
    """
    A specific capability grant with optional constraints.
    
    This represents a single capability grant that can be attached to a
    function's manifest. The grant includes the capability type and any
    constraints (e.g., which domains for NET_OUT, which paths for FS_READ).
    
    The class is frozen (immutable) to ensure grants cannot be modified
    after creation, maintaining security invariants.
    
    Attributes:
        capability: The capability type being granted
        domains: Allowed domains for NET_OUT (tuple for hashability)
        port: Port number for NET_IN
        paths: Allowed paths for FS_READ/WRITE, STATE_READ/WRITE (tuple)
        targets: Allowed IPC targets for IPC_CALL (tuple)
        scope: Scope for GPU ("render" or "compute") or SYS_PARAM
    
    Example:
        >>> grant = CapabilityGrant(
        ...     capability=Capability.NET_OUT,
        ...     domains=("api.example.com", "cdn.example.com")
        ... )
    """
    capability: Capability
    domains: Optional[tuple] = None
    port: Optional[int] = None
    paths: Optional[tuple] = None
    targets: Optional[tuple] = None
    scope: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        """
        Convert the grant to a dictionary for serialization.
        
        Returns:
            Dictionary representation of the grant
        """
        result: Dict[str, Any] = {"capability": self.capability.name}
        if self.domains:
            result["domains"] = list(self.domains)
        if self.port is not None:
            result["port"] = self.port
        if self.paths:
            result["paths"] = list(self.paths)
        if self.targets:
            result["targets"] = list(self.targets)
        if self.scope:
            result["scope"] = self.scope
        return result


@dataclass
class FunctionManifest:
    """
    Complete manifest for a registered function.
    
    The manifest contains all metadata about a function, including its
    source code, schemas, capabilities, and version information. Each
    registration creates a new immutable version.
    
    Attributes:
        name: Unique function name (e.g., "calc.add")
        version: Version number (incremented on each registration)
        runtime: Language runtime (e.g., "python3.12")
        description: Human-readable description of the function
        input_schema: JSON schema for input validation
        output_schema: JSON schema for output validation
        capabilities: Set of capability grants for this function
        source_code: The function's source code
        artifact_path: Path to compiled artifact (for compiled languages)
        build_log: Build log reference (for compiled languages)
        exposes_mcp: MCP pattern to expose (e.g., "calc.*")
        handles_event: Event pattern to handle (e.g., "task-complete.download")
        status: Current status ("cold", "warm", "running")
        created_at: Unix timestamp when this version was created
    
    Properties:
        source_hash: SHA-256 hash of the source code for integrity verification
    
    Example:
        >>> manifest = FunctionManifest(
        ...     name="calc.add",
        ...     version=1,
        ...     runtime="python3.12",
        ...     description="Adds two numbers",
        ...     input_schema={"a": "number", "b": "number"},
        ...     output_schema={"sum": "number"},
        ...     capabilities=set(),
        ...     source_code="def add(input): return {'sum': input['a'] + input['b']}"
        ... )
    """
    name: str
    version: int
    runtime: str
    description: str
    input_schema: Dict[str, Any]
    output_schema: Dict[str, Any]
    capabilities: Set[CapabilityGrant]
    source_code: str
    artifact_path: Optional[str] = None
    build_log: Optional[str] = None
    exposes_mcp: Optional[str] = None
    handles_event: Optional[str] = None
    status: str = "cold"
    created_at: float = field(default_factory=time.time)
    
    @property
    def source_hash(self) -> str:
        """
        Compute SHA-256 hash of the source code.
        
        This hash is used for:
        - Integrity verification when loading functions
        - Detecting changes between versions
        - Verifying compiled artifacts match source
        
        Returns:
            Hex-encoded SHA-256 hash of the source code
        """
        return hashlib.sha256(self.source_code.encode()).hexdigest()

    def to_dict(self) -> Dict[str, Any]:
        """
        Convert the manifest to a dictionary for serialization.
        
        Returns:
            Dictionary representation of the manifest
        """
        return {
            "name": self.name,
            "version": self.version,
            "runtime": self.runtime,
            "description": self.description,
            "input_schema": self.input_schema,
            "output_schema": self.output_schema,
            "capabilities": [c.to_dict() for c in self.capabilities],
            "source_hash": self.source_hash,
            "artifact_path": self.artifact_path,
            "build_log": self.build_log,
            "exposes_mcp": self.exposes_mcp,
            "handles_event": self.handles_event,
            "status": self.status,
            "created_at": self.created_at,
        }


@dataclass
class FunctionVersion:
    """
    A specific version of a function.
    
    Each registration creates a new FunctionVersion with an incremented
    version number. Versions are immutable once created.
    
    Attributes:
        version: Version number
        manifest: The FunctionManifest for this version
        created_at: Unix timestamp when this version was created
        last_used_at: Unix timestamp when this version was last invoked
        use_count: Number of times this version has been invoked
    
    Example:
        >>> version = FunctionVersion(
        ...     version=1,
        ...     manifest=manifest,
        ... )
    """
    version: int
    manifest: FunctionManifest
    created_at: float = field(default_factory=time.time)
    last_used_at: Optional[float] = None
    use_count: int = 0


@dataclass
class IPCLease:
    """
    A fast-path lease for direct process-to-process IPC.
    
    When a function makes repeated calls to the same target, the Router
    issues a lease that allows direct socket communication without going
    through the Router for each call. Leases are:
    
    - TTL-bound (auto-expire after timeout)
    - Capability-scoped (limited to granted capabilities)
    - Revocable (can be immediately revoked by the Router)
    
    Attributes:
        lease_id: Unique identifier for this lease
        caller_process_id: ID of the calling process
        target_name: Name of the target function
        socket_path: Path to the IPC socket
        granted_capabilities: Capabilities granted for this lease
        ttl_seconds: Time-to-live in seconds (default 300 = 5 minutes)
        created_at: Unix timestamp when the lease was created
        revoked: Whether this lease has been revoked
    
    Properties:
        is_expired: True if the lease has exceeded its TTL
    
    Example:
        >>> lease = IPCLease(
        ...     caller_process_id="proc-123",
        ...     target_name="calc.add",
        ...     socket_path="/tmp/lambda-sockets/calc.add.sock"
        ... )
    """
    lease_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    caller_process_id: str = ""
    target_name: str = ""
    socket_path: str = ""
    granted_capabilities: List[CapabilityGrant] = field(default_factory=list)
    ttl_seconds: float = 300.0
    created_at: float = field(default_factory=time.time)
    revoked: bool = False
    
    @property
    def is_expired(self) -> bool:
        """Check if the lease has exceeded its TTL."""
        return time.time() - self.created_at > self.ttl_seconds


@dataclass
class ProcessHandle:
    """
    Handle for a running function process.
    
    Each function runs in its own sandboxed process (OCI container or microVM).
    The ProcessHandle tracks the process state and provides methods for
    interaction.
    
    Attributes:
        process_id: Unique identifier for this process
        function_name: Name of the function this process runs
        function_version: Version of the function being executed
        runtime: Language runtime (e.g., "python3.12")
        pid: Operating system process ID (None if not running)
        socket_path: Path to the IPC socket for this process
        status: Current status ("starting", "running", "warm", "stopped", "error")
        created_at: Unix timestamp when this process was created
        last_heartbeat: Unix timestamp of last heartbeat from the process
        leases: List of active IPCLeases for this process
    
    Properties:
        is_warm: True if the process is running and has a valid PID
    
    Example:
        >>> handle = ProcessHandle(
        ...     function_name="calc.add",
        ...     function_version=1,
        ...     runtime="python3.12"
        ... )
    """
    process_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    function_name: str = ""
    function_version: int = 0
    runtime: str = ""
    pid: Optional[int] = None
    socket_path: Optional[str] = None
    status: str = "starting"
    created_at: float = field(default_factory=time.time)
    last_heartbeat: Optional[float] = None
    leases: List[IPCLease] = field(default_factory=list)
    
    @property
    def is_warm(self) -> bool:
        """Check if the process is running and has a valid PID."""
        return self.status == "running" and self.pid is not None
