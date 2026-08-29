"""
Lambda Execution Server - Main Entry Point.

This module provides the main LambdaServer class that ties together
all components of the Lambda Execution Server.

Architecture Reference:
    - §3.2.1 of agent-native-os-architecture.md (Lambda Execution Server)
    - §7.3 ("Lambda base images")

Version: 0.1.0
"""

import logging
from typing import Any, Dict, List

from mcp_interface import MCPControlInterface

logger = logging.getLogger(__name__)


class LambdaServer:
    """
    Lambda Execution Server.
    
    The Lambda Server provides a secure, isolated environment for executing
    functions with capability-based access control. It manages:
    
    - Function Registry: Named, described, persistent, reusable functions
    - Process Supervisor: Sandboxed function processes with warm pool
    - IPC Router: Inter-function calls with capability enforcement
    - MCP Control Interface: Tools for agent interaction
    
    Design Principles:
        1. Functions are named, described, persistent, reusable
        2. Process is the trust boundary
        3. Cross-function calls are IPC, always
        4. Capabilities are a closed, versioned power set
        5. The SDK is the only door
        6. The server exposes itself over MCP
    
    Example:
        >>> server = LambdaServer()
        >>> result = server.handle_mcp_tool("lambda.search", {"query": "calc"})
    """
    
    def __init__(self) -> None:
        """
        Initialize the Lambda Server.
        
        Creates the MCP control interface which in turn creates all
        dependent components.
        """
        self.mcp = MCPControlInterface()
        logger.info("LambdaServer initialized")
    
    def handle_mcp_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle an MCP tool call.
        
        This is the main entry point for all MCP tool invocations.
        It delegates to the MCPControlInterface for handling.
        
        Args:
            tool_name: Name of the tool (e.g., "lambda.search")
            arguments: Tool arguments as a dictionary
            
        Returns:
            Tool result as dictionary
            
        Example:
            >>> result = server.handle_mcp_tool("lambda.search", {"query": "calc"})
        """
        return self.mcp.handle_tool_call(tool_name, arguments)
    
    def get_tools(self) -> List[Dict[str, str]]:
        """
        Get available MCP tools.
        
        Returns:
            List of tool dictionaries with "name" and "description"
            
        Example:
            >>> tools = server.get_tools()
            >>> for tool in tools:
            ...     print(f"{tool['name']}: {tool['description']}")
        """
        return self.mcp.get_available_tools()
    
    def health_check(self) -> Dict[str, Any]:
        """
        Perform a health check on the server.
        
        Returns:
            Dictionary with health status and statistics
            
        Example:
            >>> health = server.health_check()
            >>> print(f"Status: {health['status']}")
        """
        stats = self.mcp.handle_tool_call("lambda.get_stats", {})
        return {
            "status": "healthy",
            "version": "0.1.0",
            "stats": stats,
        }


def create_server() -> LambdaServer:
    """
    Create and return a Lambda Server instance.
    
    This is the recommended way to create a LambdaServer.
    
    Returns:
        Configured LambdaServer instance
        
    Example:
        >>> from server import create_server
        >>> server = create_server()
        >>> result = server.handle_mcp_tool("lambda.search", {"query": "calc"})
    """
    return LambdaServer()
