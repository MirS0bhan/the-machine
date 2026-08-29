"""
IPC Router for the Lambda Execution Server.

This module routes inter-function calls with two paths:
1. Brokered call: First call to a target, or target isn't warm
2. Fast-path lease: Repeat calls to the same target

The Router enforces capability checks and maintains an audit log of all
IPC calls.

Architecture Reference:
    - §3.2.1 of agent-native-os-architecture.md (Lambda Execution Server)
    - §4 of docs/spec.md (IPC & the per-language SDK)

Version: 0.1.0
"""

import logging
import time
import uuid
from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional, Set

from models import (
    CapabilityGrant,
    FunctionManifest,
    IPCLease,
    ProcessHandle,
)

logger = logging.getLogger(__name__)


@dataclass
class IPCCallResult:
    """
    Result of an IPC call.
    
    Attributes:
        success: Whether the call succeeded
        output: Output data from the target function
        error: Error message if the call failed
        call_id: Unique identifier for this call (for auditing)
        brokered: True if call went through Router, False if fast-path
    
    Example:
        >>> result = IPCCallResult(success=True, output={"sum": 42})
        >>> if not result.success:
        ...     print(f"Call failed: {result.error}")
    """
    success: bool
    output: Any = None
    error: Optional[str] = None
    call_id: str = ""
    brokered: bool = True


class IPCRouter:
    """
    Routes inter-function calls with two paths.
    
    The IPCRouter is the central hub for inter-function communication.
    It enforces capability checks and manages fast-path leases for
    repeated calls.
    
    Two Call Paths:
        1. Brokered call: First call to a target in this process's lifetime,
           or target isn't warm. Goes through Router for capability check
           and proxy. Every brokered call is logged.
        
        2. Fast-path lease: Repeat calls to the same target (e.g., a tight
           loop, or a UI-bound handler called every frame-adjacent tick).
           Router hands back a TTL-bound, capability-scoped socket lease
           on first resolution. Subsequent calls use the leased socket
           directly, process-to-process, no Router round-trip.
    
    Security:
        - Every call is capability-checked before execution
        - Leases are TTL-bound and capability-scoped
        - Router can revoke leases immediately on manifest change
        - All calls are logged for auditing
    
    Example:
        >>> router = IPCRouter()
        >>> result = router.brokered_call(
        ...     caller_process_id="proc-123",
        ...     target_name="calc.add",
        ...     input_data={"values": [1, 2, 3]}
        ... )
    """
    
    def __init__(self) -> None:
        """
        Initialize the IPCRouter.
        
        Creates empty storage for leases, process mappings, and call logs.
        """
        # Active leases: lease_id -> IPCLease
        self._leases: Dict[str, IPCLease] = {}
        # Process ID -> list of lease IDs
        self._process_leases: Dict[str, List[str]] = {}
        # Call log for auditing
        self._call_log: List[Dict[str, Any]] = []
        # Injected components (set during server init)
        self._enforcer: Optional[Any] = None
        self._supervisor: Optional[Any] = None
        self._registry: Optional[Any] = None
        
        logger.info("IPCRouter initialized")
    
    def set_components(self, enforcer: Any, supervisor: Any, registry: Any) -> None:
        """
        Inject dependent components.
        
        Args:
            enforcer: CapabilityEnforcer instance
            supervisor: ProcessSupervisor instance
            registry: FunctionRegistry instance
        """
        self._enforcer = enforcer
        self._supervisor = supervisor
        self._registry = registry
        logger.debug("IPCRouter components injected")
    
    def brokered_call(
        self,
        caller_process_id: str,
        target_name: str,
        input_data: Any,
    ) -> IPCCallResult:
        """
        Execute a brokered IPC call.
        
        This is the standard path: check capabilities, resolve target,
        proxy input, return output. Every brokered call is logged.
        
        Args:
            caller_process_id: ID of the calling process
            target_name: Name of the target function
            input_data: Input data to send to the target
            
        Returns:
            IPCCallResult with success status and output/error
            
        Example:
            >>> result = router.brokered_call(
            ...     caller_process_id="proc-123",
            ...     target_name="calc.add",
            ...     input_data={"values": [1, 2, 3]}
            ... )
            >>> if result.success:
            ...     print(f"Result: {result.output}")
        """
        call_id = str(uuid.uuid4())
        
        # Get caller info
        caller_handle = self._supervisor.get_process(caller_process_id)
        if not caller_handle:
            logger.warning(f"Caller process {caller_process_id} not found")
            return IPCCallResult(
                success=False,
                error=f"Caller process {caller_process_id} not found",
                call_id=call_id,
                brokered=True,
            )
        
        # Get caller manifest
        caller_manifest = self._registry.get(
            caller_handle.function_name,
            caller_handle.function_version,
        )
        if not caller_manifest:
            logger.warning(f"Caller manifest not found for {caller_handle.function_name}")
            return IPCCallResult(
                success=False,
                error=f"Caller manifest not found",
                call_id=call_id,
                brokered=True,
            )
        
        # Check IPC_CALL capability
        enforcement = self._enforcer.check_ipc_call(
            caller_handle.function_name,
            target_name,
            caller_manifest,
        )
        
        if not enforcement.allowed:
            self._log_call(call_id, caller_handle.function_name, target_name, False, enforcement.reason)
            return IPCCallResult(
                success=False,
                error=enforcement.reason,
                call_id=call_id,
                brokered=True,
            )
        
        # Get or spawn target process
        target_manifest = self._registry.get(target_name)
        if not target_manifest:
            self._log_call(call_id, caller_handle.function_name, target_name, False, "Target not found")
            return IPCCallResult(
                success=False,
                error=f"Target function {target_name} not found",
                call_id=call_id,
                brokered=True,
            )
        
        target_handle = self._supervisor.get_warm(target_name)
        if not target_handle:
            # Spawn new process
            target_handle = self._supervisor.spawn(target_manifest)
        
        # Execute the call (simulate for now)
        output = self._execute_call(target_handle, input_data)
        
        # Log successful call
        self._log_call(call_id, caller_handle.function_name, target_name, True)
        
        # Issue lease for future calls
        lease = self._issue_lease(
            caller_process_id,
            target_name,
            enforcement.granted_caps or set(),
        )
        
        logger.info(
            f"Brokered call {call_id}: {caller_handle.function_name} -> {target_name}"
        )
        
        return IPCCallResult(
            success=True,
            output=output,
            call_id=call_id,
            brokered=True,
        )
    
    def fast_path_call(
        self,
        lease_id: str,
        input_data: Any,
    ) -> IPCCallResult:
        """
        Execute a call using a fast-path lease.
        
        Uses the pre-authorized socket directly, no Router round-trip.
        Falls back to brokered call if the lease has expired or the
        target is no longer warm.
        
        Args:
            lease_id: ID of the lease to use
            input_data: Input data to send to the target
            
        Returns:
            IPCCallResult with success status and output/error
            
        Example:
            >>> result = router.fast_path_call(
            ...     lease_id="lease-456",
            ...     input_data={"values": [4, 5, 6]}
            ... )
        """
        lease = self._leases.get(lease_id)
        if not lease:
            logger.warning(f"Lease {lease_id} not found")
            return IPCCallResult(
                success=False,
                error=f"Lease {lease_id} not found",
                brokered=False,
            )
        
        if lease.is_expired or lease.revoked:
            del self._leases[lease_id]
            return IPCCallResult(
                success=False,
                error="Lease expired or revoked",
                brokered=False,
            )
        
        # Get target process
        target_handle = self._supervisor.get_warm(lease.target_name)
        if not target_handle:
            # Fall back to brokered call
            logger.debug(f"Fast-path target not warm, falling back to brokered")
            return self.brokered_call(
                lease.caller_process_id,
                lease.target_name,
                input_data,
            )
        
        # Execute via direct IPC (simulate for now)
        output = self._execute_call(target_handle, input_data)
        
        logger.debug(f"Fast-path call via lease {lease_id[:8]}...")
        
        return IPCCallResult(
            success=True,
            output=output,
            brokered=False,
        )
    
    def _issue_lease(
        self,
        caller_process_id: str,
        target_name: str,
        granted_caps: Set[CapabilityGrant],
    ) -> IPCLease:
        """
        Issue a fast-path lease for direct IPC.
        
        Args:
            caller_process_id: ID of the calling process
            target_name: Name of the target function
            granted_caps: Capabilities granted for this lease
            
        Returns:
            The newly created IPCLease
        """
        lease = IPCLease(
            caller_process_id=caller_process_id,
            target_name=target_name,
            granted_capabilities=list(granted_caps),
            ttl_seconds=300.0,  # 5 minutes
        )
        
        self._leases[lease.lease_id] = lease
        
        # Track by process
        if caller_process_id not in self._process_leases:
            self._process_leases[caller_process_id] = []
        self._process_leases[caller_process_id].append(lease.lease_id)
        
        logger.debug(
            f"Issued lease {lease.lease_id[:8]}... for "
            f"{caller_process_id} -> {target_name}"
        )
        
        return lease
    
    def revoke_lease(self, lease_id: str) -> bool:
        """
        Revoke a lease immediately.
        
        Args:
            lease_id: ID of the lease to revoke
            
        Returns:
            True if lease was revoked, False if not found
            
        Example:
            >>> success = router.revoke_lease("lease-123")
        """
        lease = self._leases.get(lease_id)
        if not lease:
            return False
        
        lease.revoked = True
        del self._leases[lease_id]
        
        # Remove from process tracking
        if lease.caller_process_id in self._process_leases:
            self._process_leases[lease.caller_process_id] = [
                lid for lid in self._process_leases[lease.caller_process_id]
                if lid != lease_id
            ]
        
        logger.debug(f"Revoked lease {lease_id[:8]}...")
        return True
    
    def revoke_all_for_process(self, process_id: str) -> None:
        """
        Revoke all leases for a process.
        
        Called when a process is terminated.
        
        Args:
            process_id: ID of the process
        """
        lease_ids = self._process_leases.get(process_id, []).copy()
        for lease_id in lease_ids:
            self.revoke_lease(lease_id)
        logger.debug(f"Revoked all leases for process {process_id}")
    
    def revoke_all_for_target(self, target_name: str) -> None:
        """
        Revoke all leases for a target (on manifest change).
        
        Args:
            target_name: Name of the target function
        """
        to_revoke = [
            lid for lid, lease in self._leases.items()
            if lease.target_name == target_name
        ]
        for lease_id in to_revoke:
            self.revoke_lease(lease_id)
        logger.debug(f"Revoked all leases for target {target_name}")
    
    def _execute_call(
        self,
        target_handle: ProcessHandle,
        input_data: Any,
    ) -> Any:
        """
        Execute a call to a target process.
        
        In production, this would send input via IPC socket
        and receive output. For now, simulate.
        
        Args:
            target_handle: Handle to the target process
            input_data: Input data to send
            
        Returns:
            Output from the target function
        """
        # Simulate function execution
        # In reality, this would:
        # 1. Serialize input (msgpack)
        # 2. Send over IPC socket
        # 3. Wait for response
        # 4. Deserialize output
        
        return {
            "result": f"processed by {target_handle.function_name}",
            "input_received": input_data,
        }
    
    def _log_call(
        self,
        call_id: str,
        caller: str,
        target: str,
        success: bool,
        error: Optional[str] = None,
    ) -> None:
        """
        Log an IPC call for auditing.
        
        Args:
            call_id: Unique call identifier
            caller: Caller function name
            target: Target function name
            success: Whether the call succeeded
            error: Error message if failed
        """
        self._call_log.append({
            "call_id": call_id,
            "caller": caller,
            "target": target,
            "success": success,
            "error": error,
            "timestamp": time.time(),
        })
    
    def get_call_log(
        self,
        caller: Optional[str] = None,
        target: Optional[str] = None,
        limit: int = 100,
    ) -> List[Dict[str, Any]]:
        """
        Get call log with optional filtering.
        
        Args:
            caller: Filter by caller function name
            target: Filter by target function name
            limit: Maximum number of log entries to return
            
        Returns:
            List of call log entries
            
        Example:
            >>> logs = router.get_call_log(caller="orchestrator", limit=50)
        """
        logs = self._call_log
        
        if caller:
            logs = [l for l in logs if l["caller"] == caller]
        if target:
            logs = [l for l in logs if l["target"] == target]
        
        return logs[-limit:]
    
    def get_active_leases(self) -> List[Dict[str, Any]]:
        """
        List all active leases.
        
        Returns:
            List of active lease information
            
        Example:
            >>> leases = router.get_active_leases()
            >>> for lease in leases:
            ...     print(f"{lease['lease_id'][:8]}... expires at {lease['expires_at']}")
        """
        return [
            {
                "lease_id": lease.lease_id,
                "caller_process_id": lease.caller_process_id,
                "target_name": lease.target_name,
                "created_at": lease.created_at,
                "expires_at": lease.created_at + lease.ttl_seconds,
                "is_expired": lease.is_expired,
            }
            for lease in self._leases.values()
            if not lease.revoked and not lease.is_expired
        ]
    
    def cleanup_expired_leases(self) -> None:
        """Remove expired leases."""
        expired = [
            lid for lid, lease in self._leases.items()
            if lease.is_expired
        ]
        for lid in expired:
            del self._leases[lid]
        if expired:
            logger.debug(f"Cleaned up {len(expired)} expired leases")
