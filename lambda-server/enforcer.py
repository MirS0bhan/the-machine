"""
Capability Enforcer for the Lambda Execution Server.

This module implements the capability enforcement system, which validates
and enforces capability grants for functions. It provides two enforcement
layers:

1. Manifest validation: Ensures requested capabilities are a legal subset
   of the closed capability set (CAPS power set)
2. Runtime enforcement: Checks specific operations against granted capabilities

Key Features:
    - Closed, versioned capability set validation
    - IPC call graph edge verification
    - Filesystem and state access control
    - Network access control
    - Capability preset expansion

Architecture Reference:
    - §3.2.1 of agent-native-os-architecture.md (Lambda Execution Server)
    - §2 of docs/spec.md (Capability model — the CAPS power set)

Version: 0.1.0
"""

import logging
from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional, Set

from models import (
    Capability,
    CapabilityGrant,
    CapabilityPreset,
    FunctionManifest,
)

logger = logging.getLogger(__name__)


@dataclass
class EnforcementResult:
    """
    Result of a capability enforcement check.
    
    Attributes:
        allowed: Whether the operation is allowed
        reason: Human-readable reason if denied
        granted_caps: The capabilities that were granted (if allowed)
    
    Example:
        >>> result = EnforcementResult(allowed=True, granted_caps={grant})
        >>> if not result.allowed:
        ...     print(f"Denied: {result.reason}")
    """
    allowed: bool
    reason: Optional[str] = None
    granted_caps: Optional[Set[CapabilityGrant]] = None


class CapabilityEnforcer:
    """
    Enforces capability grants for functions.
    
    The CapabilityEnforcer implements the security model for inter-function
    communication and resource access. It validates that:
    
    1. Functions only request capabilities from the closed CAPS set
    2. Each capability has valid constraints (e.g., NET_OUT requires domains)
    3. IPC calls are only made to declared targets
    4. Filesystem and state access is limited to granted paths
    
    Two enforcement layers, deliberately redundant:
        - Kernel-level: seccomp filter + namespaces + cgroups (not implemented here)
        - SDK-level: refuses to even attempt unauthorized calls (this module)
    
    Example:
        >>> enforcer = CapabilityEnforcer()
        >>> result = enforcer.validate_manifest(manifest)
        >>> if result.allowed:
        ...     # Proceed with function registration
        ...     pass
    """
    
    # Valid capability presets (maps preset to expanded capability set)
    PRESETS: Dict[CapabilityPreset, Set[Capability]] = {
        CapabilityPreset.PURE: set(),
        CapabilityPreset.READER: {Capability.STATE_READ, Capability.FS_READ},
        CapabilityPreset.NETWORKED: {Capability.NET_OUT},
    }
    
    def __init__(self) -> None:
        """
        Initialize the CapabilityEnforcer.
        
        Creates empty storage for process grants and injects a manifest
        getter function.
        """
        # Track active grants per process (process_id -> set of grants)
        self._process_grants: Dict[str, Set[CapabilityGrant]] = {}
        # Function to get manifest (injected by MCPControlInterface)
        self._get_manifest: Callable[[str], Optional[FunctionManifest]] = lambda name: None
        
        logger.info("CapabilityEnforcer initialized")
    
    def validate_manifest(self, manifest: FunctionManifest) -> EnforcementResult:
        """
        Validate that a function's manifest declares a legal capability subset.
        
        This is the first validation layer: it ensures the manifest's
        capabilities are all from the closed CAPS set and have valid constraints.
        
        Args:
            manifest: The FunctionManifest to validate
            
        Returns:
            EnforcementResult indicating if the manifest is valid
            
        Example:
            >>> result = enforcer.validate_manifest(manifest)
            >>> if not result.allowed:
            ...     print(f"Invalid manifest: {result.reason}")
        """
        # All capabilities must be from the closed set
        for grant in manifest.capabilities:
            if not isinstance(grant.capability, Capability):
                logger.warning(f"Unknown capability in manifest: {grant.capability}")
                return EnforcementResult(
                    allowed=False,
                    reason=f"Unknown capability: {grant.capability}"
                )
            
            # Validate specific capability constraints
            validation = self._validate_grant_constraints(grant)
            if not validation.allowed:
                return validation
        
        logger.debug(f"Manifest for '{manifest.name}' validated successfully")
        return EnforcementResult(
            allowed=True,
            granted_caps=manifest.capabilities
        )
    
    def _validate_grant_constraints(self, grant: CapabilityGrant) -> EnforcementResult:
        """
        Validate constraints for a specific capability grant.
        
        Each capability has specific requirements for its constraints.
        For example, NET_OUT requires a domains list, IPC_CALL requires targets.
        
        Args:
            grant: The CapabilityGrant to validate
            
        Returns:
            EnforcementResult indicating if the constraints are valid
        """
        cap = grant.capability
        
        if cap == Capability.NET_OUT:
            if not grant.domains:
                return EnforcementResult(
                    allowed=False,
                    reason="NET_OUT requires domains list"
                )
        
        elif cap == Capability.NET_IN:
            if grant.port is None:
                return EnforcementResult(
                    allowed=False,
                    reason="NET_IN requires port"
                )
        
        elif cap in (Capability.FS_READ, Capability.FS_WRITE):
            if not grant.paths:
                return EnforcementResult(
                    allowed=False,
                    reason=f"{cap.name} requires paths list"
                )
        
        elif cap in (Capability.STATE_READ, Capability.STATE_WRITE):
            if not grant.paths:
                return EnforcementResult(
                    allowed=False,
                    reason=f"{cap.name} requires paths list"
                )
        
        elif cap == Capability.IPC_CALL:
            if not grant.targets:
                return EnforcementResult(
                    allowed=False,
                    reason="IPC_CALL requires targets list"
                )
        
        elif cap == Capability.GPU:
            if grant.scope not in ("render", "compute"):
                return EnforcementResult(
                    allowed=False,
                    reason="GPU scope must be 'render' or 'compute'"
                )
        
        return EnforcementResult(allowed=True)
    
    def expand_preset(self, preset_name: str) -> Set[CapabilityGrant]:
        """
        Expand a capability preset name to its capability set.
        
        Presets are sugar; the Broker validates the expanded set, not the
        preset name. This allows the agent to use shorthand like "pure"
        instead of manually specifying an empty capability set.
        
        Args:
            preset_name: Name of the preset (e.g., "pure", "reader", "networked")
            
        Returns:
            Set of CapabilityGrant objects for the preset
            
        Example:
            >>> grants = enforcer.expand_preset("pure")
            >>> assert len(grants) == 0  # Pure has no capabilities
            >>> grants = enforcer.expand_preset("reader")
            >>> assert any(g.capability == Capability.STATE_READ for g in grants)
        """
        try:
            preset = CapabilityPreset[preset_name.upper()]
        except KeyError:
            logger.warning(f"Unknown capability preset: {preset_name}")
            # Unknown preset, return empty set
            return set()
        
        caps = self.PRESETS.get(preset, set())
        return {CapabilityGrant(capability=cap) for cap in caps}
    
    def check_ipc_call(
        self,
        caller_name: str,
        target_name: str,
        caller_manifest: FunctionManifest,
    ) -> EnforcementResult:
        """
        Check if a caller is allowed to call a target via IPC.
        
        This is the core check: CAP_IPC_CALL is a declared call-graph edge.
        If x's manifest lists CAP_IPC_CALL(targets=[y]) and x's code tries
        to call z, the Enforcer rejects it before a socket is even opened.
        
        Args:
            caller_name: Name of the calling function
            target_name: Name of the target function
            caller_manifest: Manifest of the calling function
            
        Returns:
            EnforcementResult indicating if the IPC call is allowed
            
        Example:
            >>> result = enforcer.check_ipc_call("x", "y", manifest_x)
            >>> if result.allowed:
            ...     # Proceed with IPC call
            ...     pass
        """
        # Find IPC_CALL grants for the caller
        ipc_grants = [
            g for g in caller_manifest.capabilities
            if g.capability == Capability.IPC_CALL
        ]
        
        if not ipc_grants:
            logger.warning(f"Function '{caller_name}' has no IPC_CALL capability")
            return EnforcementResult(
                allowed=False,
                reason=f"Function {caller_name} has no IPC_CALL capability"
            )
        
        # Check if target is in any grant's targets list
        for grant in ipc_grants:
            if grant.targets and target_name in grant.targets:
                logger.debug(f"IPC call from '{caller_name}' to '{target_name}' allowed")
                return EnforcementResult(
                    allowed=True,
                    granted_caps={grant}
                )
        
        logger.warning(
            f"IPC call from '{caller_name}' to '{target_name}' denied: "
            f"target not in declared call graph"
        )
        return EnforcementResult(
            allowed=False,
            reason=f"Function {caller_name} not allowed to call {target_name}"
        )
    
    def check_state_access(
        self,
        function_name: str,
        path: str,
        write: bool = False,
    ) -> EnforcementResult:
        """
        Check if a function can read/write state at a path.
        
        Args:
            function_name: Name of the function requesting access
            path: State path to access
            write: True for write access, False for read access
            
        Returns:
            EnforcementResult indicating if access is allowed
        """
        manifest = self._get_manifest(function_name)
        if not manifest:
            return EnforcementResult(
                allowed=False,
                reason=f"Function {function_name} not found"
            )
        
        cap_type = Capability.STATE_WRITE if write else Capability.STATE_READ
        
        # Find matching grants
        for grant in manifest.capabilities:
            if grant.capability == cap_type:
                if grant.paths:
                    # Check if path matches any granted path
                    for granted_path in grant.paths:
                        if path.startswith(granted_path) or granted_path == "*":
                            return EnforcementResult(allowed=True)
        
        return EnforcementResult(
            allowed=False,
            reason=f"Function {function_name} not allowed to {'write' if write else 'read'} state at {path}"
        )
    
    def check_fs_access(
        self,
        function_name: str,
        path: str,
        write: bool = False,
    ) -> EnforcementResult:
        """
        Check if a function can read/write filesystem at a path.
        
        Args:
            function_name: Name of the function requesting access
            path: Filesystem path to access
            write: True for write access, False for read access
            
        Returns:
            EnforcementResult indicating if access is allowed
        """
        manifest = self._get_manifest(function_name)
        if not manifest:
            return EnforcementResult(
                allowed=False,
                reason=f"Function {function_name} not found"
            )
        
        cap_type = Capability.FS_WRITE if write else Capability.FS_READ
        
        for grant in manifest.capabilities:
            if grant.capability == cap_type:
                if grant.paths:
                    for granted_path in grant.paths:
                        if path.startswith(granted_path) or granted_path == "*":
                            return EnforcementResult(allowed=True)
        
        return EnforcementResult(
            allowed=False,
            reason=f"Function {function_name} not allowed to {'write' if write else 'read'} filesystem at {path}"
        )
    
    def check_network_out(
        self,
        function_name: str,
        domain: str,
    ) -> EnforcementResult:
        """
        Check if a function can make outbound network requests to a domain.
        
        Args:
            function_name: Name of the function requesting access
            domain: Domain to access
            
        Returns:
            EnforcementResult indicating if access is allowed
        """
        manifest = self._get_manifest(function_name)
        if not manifest:
            return EnforcementResult(
                allowed=False,
                reason=f"Function {function_name} not found"
            )
        
        for grant in manifest.capabilities:
            if grant.capability == Capability.NET_OUT:
                if grant.domains:
                    for granted_domain in grant.domains:
                        if domain.endswith(granted_domain) or granted_domain == "*":
                            return EnforcementResult(allowed=True)
        
        return EnforcementResult(
            allowed=False,
            reason=f"Function {function_name} not allowed network access to {domain}"
        )
    
    def register_process_grants(
        self,
        process_id: str,
        grants: Set[CapabilityGrant],
    ) -> None:
        """
        Register capability grants for a process at spawn time.
        
        This is called when a process is spawned to track what capabilities
        it has been granted.
        
        Args:
            process_id: Unique identifier for the process
            grants: Set of capability grants for this process
        """
        self._process_grants[process_id] = grants
        logger.debug(f"Registered {len(grants)} grants for process {process_id}")
    
    def get_process_grants(self, process_id: str) -> Set[CapabilityGrant]:
        """
        Get capability grants for a process.
        
        Args:
            process_id: Unique identifier for the process
            
        Returns:
            Set of capability grants for the process
        """
        return self._process_grants.get(process_id, set())
    
    def revoke_process(self, process_id: str) -> None:
        """
        Revoke all grants for a process (on termination).
        
        Args:
            process_id: Unique identifier for the process
        """
        self._process_grants.pop(process_id, None)
        logger.debug(f"Revoked all grants for process {process_id}")


def create_grant(
    capability: str,
    domains: Optional[List[str]] = None,
    port: Optional[int] = None,
    paths: Optional[List[str]] = None,
    targets: Optional[List[str]] = None,
    scope: Optional[str] = None,
) -> CapabilityGrant:
    """
    Helper to create a CapabilityGrant from string arguments.
    
    This function converts string-based capability specifications into
    proper CapabilityGrant objects, converting lists to tuples for hashability.
    
    Args:
        capability: Capability name (e.g., "IPC_CALL", "NET_OUT")
        domains: Allowed domains for NET_OUT
        port: Port for NET_IN
        paths: Allowed paths for FS_READ/WRITE, STATE_READ/WRITE
        targets: Allowed IPC targets for IPC_CALL
        scope: Scope for GPU or SYS_PARAM
        
    Returns:
        CapabilityGrant object
        
    Example:
        >>> grant = create_grant(
        ...     "IPC_CALL",
        ...     targets=["calc.add", "calc.multiply"]
        ... )
    """
    cap = Capability[capability.upper()]
    return CapabilityGrant(
        capability=cap,
        domains=tuple(domains) if domains else None,
        port=port,
        paths=tuple(paths) if paths else None,
        targets=tuple(targets) if targets else None,
        scope=scope,
    )


def parse_capabilities(cap_list: List[Dict[str, Any]]) -> Set[CapabilityGrant]:
    """
    Parse a list of capability dicts into CapabilityGrant objects.
    
    Args:
        cap_list: List of dictionaries describing capabilities
        
    Returns:
        Set of CapabilityGrant objects
        
    Example:
        >>> grants = parse_capabilities([
        ...     {"capability": "IPC_CALL", "targets": ["y"]},
        ...     {"capability": "NET_OUT", "domains": ["api.example.com"]}
        ... ])
    """
    grants: Set[CapabilityGrant] = set()
    for cap_dict in cap_list:
        grant = create_grant(**cap_dict)
        grants.add(grant)
    return grants
