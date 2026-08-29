"""
MCP Control Interface for the Lambda Execution Server.

This module implements the MCP (Model Context Protocol) control surface
that the Agent Core uses to interact with the Lambda Server. It exposes
tools for searching, registering, invoking, and managing functions.

Architecture Reference:
    - §7 of docs/spec.md (MCP control surface)
    - auil-asl-spec.md §8 (MCP as a routing fabric)

Version: 0.1.0
"""

import logging
import uuid
from typing import Any, Callable, Dict, List, Optional, Set

from models import CapabilityGrant, FunctionManifest
from registry import FunctionRegistry
from enforcer import CapabilityEnforcer, parse_capabilities
from supervisor import ProcessSupervisor
from router import IPCRouter
from executor import LocalExecutor

logger = logging.getLogger(__name__)


class MCPControlInterface:
    """
    MCP control surface for the Lambda Server.
    
    This is the interface the Agent Core actually talks to — the Lambda
    Server is, from the Bus's point of view, just another MCP server.
    
    Available Tools:
        - lambda.search: Search for functions by keyword
        - lambda.describe: Get full function manifest
        - lambda.register: Create or update a function
        - lambda.invoke: Directly invoke a function
        - lambda.deprecate: Mark a version as deprecated
        - lambda.rollback: Rollback to a previous version
        - lambda.list_calls: Introspect IPC call graph
        - lambda.list_functions: List all registered functions
        - lambda.list_processes: List running processes
        - lambda.list_warm_pool: List warm pool entries
        - lambda.get_call_log: Get IPC call log
        - lambda.get_stats: Get server statistics
    
    Example:
        >>> mcp = MCPControlInterface()
        >>> result = mcp.handle_tool_call("lambda.search", {"query": "calc"})
    """
    
    def __init__(self) -> None:
        """
        Initialize the MCPControlInterface.
        
        Creates all dependent components and connects them together.
        """
        self.registry = FunctionRegistry()
        self.enforcer = CapabilityEnforcer()
        self.supervisor = ProcessSupervisor()
        self.router = IPCRouter()
        
        # Connect components
        self.router.set_components(self.enforcer, self.supervisor, self.registry)
        self.enforcer._get_manifest = lambda name: self.registry.get(name)
        
        # Register MCP tools
        self._tools: Dict[str, Callable[[Dict[str, Any]], Dict[str, Any]]] = {
            "lambda.search": self._handle_search,
            "lambda.describe": self._handle_describe,
            "lambda.register": self._handle_register,
            "lambda.invoke": self._handle_invoke,
            "lambda.deprecate": self._handle_deprecate,
            "lambda.rollback": self._handle_rollback,
            "lambda.list_calls": self._handle_list_calls,
            "lambda.list_functions": self._handle_list_functions,
            "lambda.list_processes": self._handle_list_processes,
            "lambda.list_warm_pool": self._handle_list_warm_pool,
            "lambda.get_call_log": self._handle_get_call_log,
            "lambda.get_stats": self._handle_get_stats,
        }
        
        logger.info(f"MCPControlInterface initialized with {len(self._tools)} tools")
    
    def handle_tool_call(self, tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle an MCP tool call.
        
        This is the main entry point for all MCP tool invocations.
        It routes the call to the appropriate handler and returns
        the result as a dictionary.
        
        Args:
            tool_name: Name of the tool (e.g., "lambda.search")
            arguments: Tool arguments as a dictionary
            
        Returns:
            Tool result as dictionary
            
        Example:
            >>> result = mcp.handle_tool_call("lambda.search", {"query": "calc"})
            >>> print(result["results"])
        """
        handler = self._tools.get(tool_name)
        if not handler:
            logger.warning(f"Unknown tool: {tool_name}")
            return {
                "error": f"Unknown tool: {tool_name}",
                "available_tools": list(self._tools.keys()),
            }
        
        try:
            logger.debug(f"Handling tool call: {tool_name}")
            return handler(arguments)
        except Exception as e:
            logger.error(f"Error handling {tool_name}: {e}", exc_info=True)
            return {
                "error": str(e),
                "tool": tool_name,
            }
    
    def _handle_search(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.search(query) - Semantic/keyword search over registry.
        
        Returns candidate functions with name, description, and schemas.
        This is the "is there already a function for this" step.
        
        Args:
            args: Must contain "query" key
            
        Returns:
            Dictionary with "results" list and "count"
        """
        query = args.get("query", "")
        if not query:
            return {"error": "query is required"}
        
        results = self.registry.search(query)
        return {
            "results": results,
            "count": len(results),
        }
    
    def _handle_describe(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.describe(name) - Full manifest for one function.
        
        Returns complete function info including capabilities and version history.
        
        Args:
            args: Must contain "name" key
            
        Returns:
            Dictionary with "manifest" and "history"
        """
        name = args.get("name")
        if not name:
            return {"error": "name is required"}
        
        manifest = self.registry.get(name)
        if not manifest:
            return {"error": f"Function {name} not found"}
        
        history = self.registry.get_version_history(name)
        
        return {
            "manifest": manifest.to_dict(),
            "history": history,
        }
    
    def _handle_register(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.register(...) - Create or update a function.
        
        Triggers capability validation, build (if compiled),
        sandbox profile derivation, and Registry entry.
        
        Args:
            args: Must contain "name", "runtime", "code", "description"
                  May contain "input_schema", "output_schema", "capabilities",
                  "exposes_mcp"
            
        Returns:
            Dictionary with "success" and "manifest"
        """
        # Extract required args
        name = args.get("name")
        runtime = args.get("runtime")
        code = args.get("code")
        description = args.get("description")
        input_schema = args.get("input_schema", {})
        output_schema = args.get("output_schema", {})
        exposes_mcp = args.get("exposes_mcp")
        handles_event = args.get("handles_event")
        
        if not all([name, runtime, code, description]):
            return {"error": "name, runtime, code, and description are required"}
        
        # Parse capabilities
        capabilities_raw = args.get("capabilities", [])
        if isinstance(capabilities_raw, str):
            # Handle preset name
            capabilities = self.enforcer.expand_preset(capabilities_raw)
        elif isinstance(capabilities_raw, list):
            capabilities = parse_capabilities(capabilities_raw)
        else:
            capabilities = set()
        
        # Create temporary manifest for validation
        temp_manifest = FunctionManifest(
            name=name,
            version=1,
            runtime=runtime,
            description=description,
            input_schema=input_schema,
            output_schema=output_schema,
            capabilities=capabilities,
            source_code=code,
        )
        
        # Validate capabilities
        validation = self.enforcer.validate_manifest(temp_manifest)
        if not validation.allowed:
            return {"error": f"Capability validation failed: {validation.reason}"}
        
        # Register the function
        manifest = self.registry.register(
            name=name,
            runtime=runtime,
            code=code,
            description=description,
            input_schema=input_schema,
            output_schema=output_schema,
            capabilities=capabilities,
            exposes_mcp=exposes_mcp,
            handles_event=handles_event,
        )
        
        logger.info(f"Registered function '{name}' version {manifest.version}")
        
        return {
            "success": True,
            "manifest": manifest.to_dict(),
        }
    
    def _handle_invoke(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.invoke(name, input) - Direct invocation.
        
        Used when the agent wants a result immediately rather than
        through a UI-bound intent.
        
        Args:
            args: Must contain "name" key, may contain "input"
            
        Returns:
            Dictionary with "success", "output", and "call_id"
        """
        name = args.get("name")
        input_data = args.get("input", {})
        
        if not name:
            return {"error": "name is required"}
        
        # Check if function exists
        manifest = self.registry.get(name)
        if not manifest:
            return {"success": False, "error": f"Function {name} not found"}
        
        # For direct invocation, execute the function directly using the
        # local sandboxed executor. In production, pure/local functions may
        # also run inside the ProcessSupervisor's isolated processes.
        call_id = str(uuid.uuid4())
        
        try:
            executor = LocalExecutor()
            result = executor.execute(manifest, input_data)
        except Exception as e:
            logger.error(f"Execution of '{name}' failed: {e}", exc_info=True)
            return {"success": False, "error": str(e), "call_id": call_id}
        
        logger.info(
            f"Invoked function '{name}' (call_id={call_id}, "
            f"{result.duration_ms:.1f}ms)"
        )
        
        return {
            "success": True,
            "output": result.output,
            "call_id": call_id,
            "duration_ms": result.duration_ms,
        }
    
    def _handle_deprecate(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.deprecate(name, version) - Mark version as deprecated.
        
        Args:
            args: Must contain "name" and "version" keys
            
        Returns:
            Dictionary with "success" key
        """
        name = args.get("name")
        version = args.get("version")
        
        if not name or version is None:
            return {"error": "name and version are required"}
        
        success = self.registry.deprecate(name, version)
        return {"success": success}
    
    def _handle_rollback(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.rollback(name, version) - Rollback to previous version.
        
        Creates a new version with the old code, maintaining immutable
        version history.
        
        Args:
            args: Must contain "name" and "version" keys
            
        Returns:
            Dictionary with "success" and "manifest"
        """
        name = args.get("name")
        version = args.get("version")
        
        if not name or version is None:
            return {"error": "name and version are required"}
        
        manifest = self.registry.rollback(name, version)
        if manifest:
            return {
                "success": True,
                "manifest": manifest.to_dict(),
            }
        else:
            return {"error": f"Rollback to version {version} failed"}
    
    def _handle_list_calls(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.list_calls(name) - Introspect function's IPC call graph.
        
        Args:
            args: Must contain "name" key
            
        Returns:
            Dictionary with "function", "calls", and "count"
        """
        name = args.get("name")
        if not name:
            return {"error": "name is required"}
        
        calls = self.registry.list_calls(name)
        return {
            "function": name,
            "calls": calls,
            "count": len(calls),
        }
    
    def _handle_list_functions(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.list_functions() - List all registered functions.
        
        Returns:
            Dictionary with "functions" list and "count"
        """
        functions = self.registry.list_functions()
        return {
            "functions": functions,
            "count": len(functions),
        }
    
    def _handle_list_processes(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.list_processes() - List all running processes.
        
        Returns:
            Dictionary with "processes" list and "count"
        """
        processes = self.supervisor.list_processes()
        return {
            "processes": processes,
            "count": len(processes),
        }
    
    def _handle_list_warm_pool(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.list_warm_pool() - List warm pool entries.
        
        Returns:
            Dictionary with "warm_pool" mapping
        """
        pool = self.supervisor.list_warm_pool()
        return {"warm_pool": pool}
    
    def _handle_get_call_log(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.get_call_log() - Get IPC call log.
        
        Args:
            args: May contain "caller", "target", "limit" keys
            
        Returns:
            Dictionary with "logs" list and "count"
        """
        caller = args.get("caller")
        target = args.get("target")
        limit = args.get("limit", 100)
        
        logs = self.router.get_call_log(caller, target, limit)
        return {
            "logs": logs,
            "count": len(logs),
        }
    
    def _handle_get_stats(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle lambda.get_stats() - Get server statistics.
        
        Returns:
            Dictionary with various statistics
        """
        stats = self.supervisor.get_stats()
        stats["leases_active"] = len(self.router.get_active_leases())
        stats["functions_registered"] = len(self.registry.list_functions())
        return stats
    
    def get_available_tools(self) -> List[Dict[str, str]]:
        """
        Get list of available MCP tools.
        
        Returns:
            List of tool dictionaries with "name" and "description"
            
        Example:
            >>> tools = mcp.get_available_tools()
            >>> for tool in tools:
            ...     print(f"{tool['name']}: {tool['description']}")
        """
        tools: List[Dict[str, str]] = []
        for name in self._tools.keys():
            tools.append({
                "name": name,
                "description": self._get_tool_description(name),
            })
        return tools
    
    def _get_tool_description(self, tool_name: str) -> str:
        """
        Get description for a tool.
        
        Args:
            tool_name: Name of the tool
            
        Returns:
            Human-readable description
        """
        descriptions = {
            "lambda.search": "Search for functions by keyword or semantic query",
            "lambda.describe": "Get full manifest for a function",
            "lambda.register": "Create or update a function",
            "lambda.invoke": "Directly invoke a function",
            "lambda.deprecate": "Mark a function version as deprecated",
            "lambda.rollback": "Rollback to a previous function version",
            "lambda.list_calls": "Introspect a function's IPC call graph",
            "lambda.list_functions": "List all registered functions",
            "lambda.list_processes": "List all running processes",
            "lambda.list_warm_pool": "List warm pool entries",
            "lambda.get_call_log": "Get IPC call log",
            "lambda.get_stats": "Get server statistics",
        }
        return descriptions.get(tool_name, "No description available")
