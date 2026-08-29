"""
Process Supervisor for the Lambda Execution Server.

This module manages sandboxed function processes, including warm pool
management, process lifecycle, and resource isolation.

Key Features:
    - One process per function (or one warm-pool slot)
    - Warm vs cold based on frequency/latency needs
    - OCI container or microVM isolation with seccomp + namespaces + cgroups
    - Automatic cleanup of stale warm pool entries
    - Health monitoring via heartbeats

Architecture Reference:
    - §3.2.1 of agent-native-os-architecture.md (Lambda Execution Server)
    - §3 of docs/spec.md (Process & warm pool model)

Version: 0.1.0
"""

import logging
import os
import time
import uuid
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Set

from models import FunctionManifest, ProcessHandle

logger = logging.getLogger(__name__)


@dataclass
class WarmPoolConfig:
    """
    Configuration for warm pool management.
    
    Attributes:
        max_warm_per_function: Maximum warm processes per function
        warm_timeout_seconds: How long a warm process can idle before cleanup
        heartbeat_interval: Expected heartbeat interval in seconds
        max_total_warm: Maximum total warm processes across all functions
    
    Example:
        >>> config = WarmPoolConfig(
        ...     max_warm_per_function=3,
        ...     warm_timeout_seconds=600.0
        ... )
    """
    max_warm_per_function: int = 2
    warm_timeout_seconds: float = 300.0  # 5 minutes
    heartbeat_interval: float = 30.0
    max_total_warm: int = 50


class ProcessSupervisor:
    """
    Manages sandboxed function processes.
    
    The ProcessSupervisor is responsible for:
    - Spawning new processes for functions
    - Managing warm pools for frequently-used functions
    - Killing stale or unhealthy processes
    - Tracking process health via heartbeats
    
    Process Isolation:
        In production, each process runs in an isolated environment:
        - OCI container or microVM
        - seccomp filter based on capabilities
        - Network, mount, and PID namespaces
        - cgroup resource limits
    
    Warm Pool:
        Frequently-hit or latency-sensitive functions stay warm.
        The supervisor manages a pool of warm processes that can
        be reused without cold-start overhead.
    
    Example:
        >>> supervisor = ProcessSupervisor()
        >>> handle = supervisor.spawn(manifest)
        >>> # ... use the process ...
        >>> supervisor.add_to_warm_pool(handle)
    """
    
    def __init__(self, config: Optional[WarmPoolConfig] = None) -> None:
        """
        Initialize the ProcessSupervisor.
        
        Args:
            config: Optional configuration for warm pool management
        """
        self.config = config or WarmPoolConfig()
        # function_name -> list of ProcessHandle (warm pool)
        self._warm_pool: Dict[str, List[ProcessHandle]] = {}
        # process_id -> ProcessHandle
        self._processes: Dict[str, ProcessHandle] = {}
        # Track last health check times (process_id -> timestamp)
        self._health_checks: Dict[str, float] = {}
        
        logger.info(
            f"ProcessSupervisor initialized: "
            f"max_warm_per_function={self.config.max_warm_per_function}, "
            f"warm_timeout={self.config.warm_timeout_seconds}s"
        )
    
    def spawn(
        self,
        manifest: FunctionManifest,
        capability_grants: Optional[Set[Any]] = None,
    ) -> ProcessHandle:
        """
        Spawn a new process for a function.
        
        First checks warm pool, then spawns new if needed.
        
        Args:
            manifest: FunctionManifest describing the function to run
            capability_grants: Optional capability grants for the process
            
        Returns:
            ProcessHandle for the spawned process
            
        Example:
            >>> handle = supervisor.spawn(manifest)
            >>> print(f"Process {handle.process_id} started")
        """
        # Check warm pool first
        if manifest.name in self._warm_pool:
            warm = self._warm_pool[manifest.name]
            if warm:
                # Reuse warm process
                handle = warm.pop(0)
                handle.function_version = manifest.version
                handle.status = "running"
                self._update_heartbeat(handle)
                logger.info(f"Reusing warm process {handle.process_id} for '{manifest.name}'")
                return handle
        
        # Spawn new process
        handle = ProcessHandle(
            function_name=manifest.name,
            function_version=manifest.version,
            runtime=manifest.runtime,
            status="starting",
        )
        
        # Create IPC socket path
        socket_dir = os.environ.get("LAMBDA_SOCKET_DIR", "/tmp/lambda-sockets")
        os.makedirs(socket_dir, exist_ok=True)
        handle.socket_path = os.path.join(
            socket_dir,
            f"{manifest.name}-v{manifest.version}-{handle.process_id}.sock"
        )
        
        # In a real implementation, this would:
        # 1. Create OCI container or microVM
        # 2. Apply seccomp filter based on capabilities
        # 3. Set up namespaces (network, mount, PID)
        # 4. Apply cgroup resource limits
        # 5. Start the function runtime with the code
        
        # For now, we'll simulate with a placeholder
        handle.pid = self._simulate_spawn(manifest, handle)
        handle.status = "running"
        
        self._processes[handle.process_id] = handle
        self._update_heartbeat(handle)
        
        logger.info(
            f"Spawned new process {handle.process_id} for '{manifest.name}' "
            f"(pid={handle.pid}, socket={handle.socket_path})"
        )
        
        return handle
    
    def _simulate_spawn(
        self,
        manifest: FunctionManifest,
        handle: ProcessHandle,
    ) -> int:
        """
        Simulate process spawn for development.
        
        In production, this would use container runtime (Docker, containerd,
        or Firecracker for microVMs).
        
        Args:
            manifest: Function manifest
            handle: Process handle to populate
            
        Returns:
            Simulated PID
        """
        # Simulate PID
        return os.getpid() + hash(handle.process_id) % 10000
    
    def kill(self, process_id: str) -> bool:
        """
        Kill a process and remove it from tracking.
        
        Args:
            process_id: ID of the process to kill
            
        Returns:
            True if process was killed, False if not found
            
        Example:
            >>> success = supervisor.kill("proc-123")
        """
        handle = self._processes.get(process_id)
        if not handle:
            logger.warning(f"Cannot kill: process {process_id} not found")
            return False
        
        # In production, this would kill the container/microVM
        handle.status = "stopped"
        
        # Remove from warm pool if present
        if handle.function_name in self._warm_pool:
            pool = self._warm_pool[handle.function_name]
            self._warm_pool[handle.function_name] = [
                p for p in pool if p.process_id != process_id
            ]
        
        # Clean up
        if process_id in self._processes:
            del self._processes[process_id]
        
        logger.info(f"Killed process {process_id} for '{handle.function_name}'")
        return True
    
    def get_warm(self, function_name: str) -> Optional[ProcessHandle]:
        """
        Get a warm process for a function, if available.
        
        Args:
            function_name: Name of the function
            
        Returns:
            ProcessHandle if warm process available, None otherwise
            
        Example:
            >>> warm = supervisor.get_warm("calc.add")
            >>> if warm:
            ...     # Use the warm process
            ...     pass
        """
        if function_name in self._warm_pool:
            pool = self._warm_pool[function_name]
            if pool:
                return pool[0]
        return None
    
    def add_to_warm_pool(self, handle: ProcessHandle) -> None:
        """
        Add a process to the warm pool after use.
        
        If the warm pool is full for this function, the oldest warm
        process is killed to make room.
        
        Args:
            handle: ProcessHandle to add to warm pool
            
        Example:
            >>> supervisor.add_to_warm_pool(handle)
        """
        if handle.function_name not in self._warm_pool:
            self._warm_pool[handle.function_name] = []
        
        pool = self._warm_pool[handle.function_name]
        
        # Respect max warm per function
        if len(pool) < self.config.max_warm_per_function:
            handle.status = "warm"
            pool.append(handle)
            logger.debug(
                f"Added process {handle.process_id} to warm pool for '{handle.function_name}' "
                f"({len(pool)}/{self.config.max_warm_per_function})"
            )
        else:
            # Kill oldest warm process
            oldest = pool.pop(0)
            self.kill(oldest.process_id)
            handle.status = "warm"
            pool.append(handle)
            logger.debug(
                f"Replaced oldest warm process for '{handle.function_name}'"
            )
    
    def heartbeat(self, process_id: str) -> bool:
        """
        Update heartbeat for a process.
        
        Args:
            process_id: ID of the process
            
        Returns:
            True if heartbeat updated, False if process not found
            
        Example:
            >>> success = supervisor.heartbeat("proc-123")
        """
        handle = self._processes.get(process_id)
        if not handle:
            return False
        
        self._update_heartbeat(handle)
        return True
    
    def _update_heartbeat(self, handle: ProcessHandle) -> None:
        """
        Update last heartbeat time.
        
        Args:
            handle: ProcessHandle to update
        """
        handle.last_heartbeat = time.time()
        self._health_checks[handle.process_id] = time.time()
    
    def check_health(self, process_id: str) -> bool:
        """
        Check if a process is healthy based on heartbeat.
        
        A process is considered healthy if it has sent a heartbeat
        within 2x the expected interval.
        
        Args:
            process_id: ID of the process
            
        Returns:
            True if process is healthy, False otherwise
            
        Example:
            >>> if supervisor.check_health("proc-123"):
            ...     print("Process is healthy")
        """
        last_check = self._health_checks.get(process_id, 0)
        return time.time() - last_check < self.config.heartbeat_interval * 2
    
    def cleanup_stale(self) -> None:
        """
        Clean up stale warm pool entries.
        
        Removes warm processes that have exceeded the timeout without
        a heartbeat.
        """
        now = time.time()
        killed_count = 0
        
        for function_name in list(self._warm_pool.keys()):
            pool = self._warm_pool[function_name]
            stale: List[ProcessHandle] = []
            
            for handle in pool:
                if handle.last_heartbeat:
                    age = now - handle.last_heartbeat
                    if age > self.config.warm_timeout_seconds:
                        stale.append(handle)
                else:
                    # No heartbeat yet, check creation time
                    age = now - handle.created_at
                    if age > self.config.warm_timeout_seconds:
                        stale.append(handle)
            
            # Remove stale processes
            for handle in stale:
                self.kill(handle.process_id)
                killed_count += 1
        
        if killed_count > 0:
            logger.info(f"Cleaned up {killed_count} stale warm processes")
    
    def get_process(self, process_id: str) -> Optional[ProcessHandle]:
        """
        Get a process handle by ID.
        
        Args:
            process_id: ID of the process
            
        Returns:
            ProcessHandle if found, None otherwise
            
        Example:
            >>> handle = supervisor.get_process("proc-123")
        """
        return self._processes.get(process_id)
    
    def list_processes(self) -> List[Dict[str, Any]]:
        """
        List all running processes.
        
        Returns:
            List of process metadata dictionaries
            
        Example:
            >>> processes = supervisor.list_processes()
            >>> for p in processes:
            ...     print(f"{p['process_id']}: {p['function_name']}")
        """
        result: List[Dict[str, Any]] = []
        for handle in self._processes.values():
            result.append({
                "process_id": handle.process_id,
                "function_name": handle.function_name,
                "function_version": handle.function_version,
                "runtime": handle.runtime,
                "status": handle.status,
                "pid": handle.pid,
                "socket_path": handle.socket_path,
                "created_at": handle.created_at,
                "last_heartbeat": handle.last_heartbeat,
            })
        return result
    
    def list_warm_pool(self) -> Dict[str, List[Dict[str, Any]]]:
        """
        List all warm pool entries.
        
        Returns:
            Dictionary mapping function names to lists of warm process info
            
        Example:
            >>> pool = supervisor.list_warm_pool()
            >>> for func, processes in pool.items():
            ...     print(f"{func}: {len(processes)} warm processes")
        """
        result: Dict[str, List[Dict[str, Any]]] = {}
        for function_name, pool in self._warm_pool.items():
            result[function_name] = [
                {
                    "process_id": h.process_id,
                    "version": h.function_version,
                    "status": h.status,
                }
                for h in pool
            ]
        return result
    
    def get_stats(self) -> Dict[str, Any]:
        """
        Get supervisor statistics.
        
        Returns:
            Dictionary with statistics about processes and warm pool
            
        Example:
            >>> stats = supervisor.get_stats()
            >>> print(f"Total processes: {stats['total_processes']}")
        """
        total_processes = len(self._processes)
        total_warm = sum(len(pool) for pool in self._warm_pool.values())
        running = sum(1 for h in self._processes.values() if h.status == "running")
        
        return {
            "total_processes": total_processes,
            "running": running,
            "warm_pool_total": total_warm,
            "warm_pool_by_function": {
                name: len(pool) for name, pool in self._warm_pool.items()
            },
        }
