"""
Lambda SDK for Python.

The SDK is the only door - a function's code never touches a raw socket.
It calls call("y", input); the framework decides whether that's a brokered
round-trip or a leased fast-path channel.

Architecture Reference:
    - §4 of docs/spec.md (IPC & the per-language SDK)

Usage:
    from lambda_sdk import call, state, capabilities

    @capabilities(ipc_call=["y"])
    def x(input):
        output = call("y", input)  # looks synchronous; is IPC under the hood
        return transform(output)

Version: 0.1.0
"""

import json
import logging
from typing import Any, Callable, Dict, List, Optional, Set
from functools import wraps

logger = logging.getLogger(__name__)


class LambdaContext:
    """
    Context for a function execution.
    
    The context provides information about the current execution and
    manages leases for fast-path IPC calls.
    
    Attributes:
        function_name: Name of the function being executed
        process_id: ID of the process running this function
    
    Example:
        >>> ctx = LambdaContext("calc.add", "proc-123")
        >>> lease = ctx.get_lease("calc.multiply")
    """
    
    def __init__(self, function_name: str, process_id: str) -> None:
        """
        Initialize the LambdaContext.
        
        Args:
            function_name: Name of the function being executed
            process_id: ID of the process running this function
        """
        self.function_name = function_name
        self.process_id = process_id
        self._leases: Dict[str, str] = {}  # target -> lease_id
    
    def get_lease(self, target: str) -> Optional[str]:
        """
        Get an existing lease for a target.
        
        Args:
            target: Name of the target function
            
        Returns:
            Lease ID if found, None otherwise
        """
        return self._leases.get(target)
    
    def set_lease(self, target: str, lease_id: str) -> None:
        """
        Store a lease for future calls.
        
        Args:
            target: Name of the target function
            lease_id: ID of the lease to store
        """
        self._leases[target] = lease_id


# Global context (set by runtime)
_context: Optional[LambdaContext] = None


def get_context() -> LambdaContext:
    """
    Get the current function context.
    
    Returns:
        Current LambdaContext instance
        
    Example:
        >>> ctx = get_context()
        >>> print(f"Running in {ctx.function_name}")
    """
    global _context
    if _context is None:
        _context = LambdaContext("__unknown__", "__unknown__")
    return _context


def set_context(ctx: LambdaContext) -> None:
    """
    Set the current function context.
    
    This is called by the runtime when a function is invoked.
    
    Args:
        ctx: LambdaContext to set as current
    """
    global _context
    _context = ctx


def call(target: str, input_data: Any) -> Any:
    """
    Call another function via IPC.
    
    The SDK makes this look like a normal function call, but every call
    is actually IPC underneath. The framework decides whether to use
    a brokered call or fast-path lease.
    
    Args:
        target: Name of the function to call
        input_data: Input data to send
        
    Returns:
        Output from the target function
        
    Raises:
        PermissionError: If not allowed to call target
        ConnectionError: If IPC fails
        
    Example:
        >>> output = call("calc.add", {"values": [1, 2, 3]})
        >>> print(output["sum"])
    """
    ctx = get_context()
    
    # Check for existing lease
    lease_id = ctx.get_lease(target)
    
    if lease_id:
        # Try fast-path call
        try:
            return _fast_path_call(lease_id, input_data)
        except Exception:
            # Lease might have expired, fall back to brokered
            pass
    
    # Brokered call
    return _brokered_call(ctx, target, input_data)


def _brokered_call(ctx: LambdaContext, target: str, input_data: Any) -> Any:
    """
    Execute a brokered IPC call.
    
    This goes through the Lambda Server for capability checking
    and proxying.
    
    Args:
        ctx: Current function context
        target: Name of the target function
        input_data: Input data to send
        
    Returns:
        Output from the target function
    """
    # In production, this would communicate with the Lambda Server
    # For now, simulate the call
    return {
        "result": f"called {target}",
        "input": input_data,
    }


def _fast_path_call(lease_id: str, input_data: Any) -> Any:
    """
    Execute a fast-path call using a lease.
    
    Uses the pre-authorized socket directly, no Router round-trip.
    
    Args:
        lease_id: ID of the lease to use
        input_data: Input data to send
        
    Returns:
        Output from the target function
    """
    # In production, this would use the leased socket directly
    # For now, simulate the call
    return {
        "result": f"fast-path via lease {lease_id[:8]}",
        "input": input_data,
    }


class StateAccessor:
    """
    State accessor for reading/writing to the State Store.
    
    Maps to the parent doc's State Store (§3.2.2), gated by
    CAP_STATE_READ/CAP_STATE_WRITE.
    
    Example:
        >>> value = state.get("myapp/config")
        >>> state.set("myapp/cache", {"key": "value"})
    """
    
    def __init__(self) -> None:
        """
        Initialize the StateAccessor.
        """
        self._leases: Dict[str, str] = {}  # path -> lease_id
    
    def get(self, path: str) -> Any:
        """
        Read state at a path.
        
        Args:
            path: State path to read
            
        Returns:
            State value
            
        Raises:
            PermissionError: If not allowed to read
            
        Example:
            >>> value = state.get("myapp/config")
        """
        # Check capability (simulated)
        # In production, this would go through the enforcer
        
        # Simulate state read
        return {"path": path, "value": None}
    
    def set(self, path: str, value: Any) -> bool:
        """
        Write state at a path.
        
        Args:
            path: State path to write
            value: Value to write
            
        Returns:
            True if successful
            
        Raises:
            PermissionError: If not allowed to write
            
        Example:
            >>> success = state.set("myapp/cache", {"key": "value"})
        """
        # Check capability (simulated)
        # In production, this would go through the enforcer
        
        # Simulate state write
        return True


# Global state accessor
state = StateAccessor()


def capabilities(
    ipc_call: Optional[List[str]] = None,
    fs_read: Optional[List[str]] = None,
    fs_write: Optional[List[str]] = None,
    net_out: Optional[List[str]] = None,
    net_in: Optional[int] = None,
    state_read: Optional[List[str]] = None,
    state_write: Optional[List[str]] = None,
    gpu: Optional[str] = None,
    spawn_ephemeral: bool = False,
    timer: bool = False,
) -> Callable[[Callable], Callable]:
    """
    Decorator to declare function capabilities.
    
    This is the manifest declaration that the enforcer validates.
    Capabilities are a closed, versioned power set.
    
    Args:
        ipc_call: List of function names this function can call
        fs_read: List of filesystem paths this function can read
        fs_write: List of filesystem paths this function can write
        net_out: List of domains this function can access
        net_in: Port to listen on (rare)
        state_read: List of state paths this function can read
        state_write: List of state paths this function can write
        gpu: GPU scope ("render" or "compute")
        spawn_ephemeral: Can spawn throwaway sub-processes
        timer: Can schedule itself via Event/Scheduler Bus
    
    Returns:
        Decorator function
        
    Example:
        >>> @capabilities(ipc_call=["y", "z"])
        ... def x(input):
        ...     output = call("y", input)
        ...     return transform(output)
    """
    def decorator(func: Callable) -> Callable:
        # Store capabilities as function attribute
        func._lambda_capabilities = {
            "ipc_call": ipc_call or [],
            "fs_read": fs_read or [],
            "fs_write": fs_write or [],
            "net_out": net_out or [],
            "net_in": net_in,
            "state_read": state_read or [],
            "state_write": state_write or [],
            "gpu": gpu,
            "spawn_ephemeral": spawn_ephemeral,
            "timer": timer,
        }
        
        @wraps(func)
        def wrapper(*args, **kwargs):
            return func(*args, **kwargs)
        
        return wrapper
    
    return decorator


def get_capabilities(func: Callable) -> Dict[str, Any]:
    """
    Get capabilities declared for a function.
    
    Args:
        func: Function to get capabilities for
        
    Returns:
        Dictionary of capabilities
        
    Example:
        >>> caps = get_capabilities(my_func)
        >>> print(caps["ipc_call"])
    """
    return getattr(func, "_lambda_capabilities", {})


class LambdaFunction:
    """
    Wrapper for a registered Lambda function.
    
    This is what gets serialized and sent to the Lambda Server.
    
    Attributes:
        name: Unique function name
        func: The actual function callable
        runtime: Language runtime
        description: Human-readable description
        input_schema: JSON schema for input
        output_schema: JSON schema for output
        exposes_mcp: MCP pattern to expose
        capabilities: Declared capabilities
    
    Example:
        >>> func = LambdaFunction(
        ...     name="calc.add",
        ...     func=add_function,
        ...     description="Adds two numbers"
        ... )
    """
    
    def __init__(
        self,
        name: str,
        func: Callable,
        runtime: str = "python3.12",
        description: str = "",
        input_schema: Optional[Dict[str, Any]] = None,
        output_schema: Optional[Dict[str, Any]] = None,
        exposes_mcp: Optional[str] = None,
    ) -> None:
        """
        Initialize a LambdaFunction.
        
        Args:
            name: Unique function name
            func: The actual function callable
            runtime: Language runtime
            description: Human-readable description
            input_schema: JSON schema for input
            output_schema: JSON schema for output
            exposes_mcp: MCP pattern to expose
        """
        self.name = name
        self.func = func
        self.runtime = runtime
        self.description = description or func.__doc__ or ""
        self.input_schema = input_schema or {}
        self.output_schema = output_schema or {}
        self.exposes_mcp = exposes_mcp
        
        # Get capabilities from decorator
        self.capabilities = get_capabilities(func)
    
    def to_dict(self) -> Dict[str, Any]:
        """
        Convert to dictionary for registration.
        
        Returns:
            Dictionary representation
        """
        return {
            "name": self.name,
            "runtime": self.runtime,
            "description": self.description,
            "input_schema": self.input_schema,
            "output_schema": self.output_schema,
            "capabilities": self.capabilities,
            "exposes_mcp": self.exposes_mcp,
        }
    
    def execute(self, input_data: Any) -> Any:
        """
        Execute the function.
        
        Args:
            input_data: Input data for the function
            
        Returns:
            Function output
        """
        return self.func(input_data)


def register_function(
    server_url: str,
    function: LambdaFunction,
) -> Dict[str, Any]:
    """
    Register a function with the Lambda Server.
    
    Args:
        server_url: URL of the Lambda Server
        function: LambdaFunction to register
        
    Returns:
        Registration result
        
    Example:
        >>> result = register_function(
        ...     "http://localhost:8080",
        ...     lambda_func
        ... )
    """
    # In production, this would make an HTTP/MCP call to the server
    # For now, simulate
    return {
        "success": True,
        "name": function.name,
    }
