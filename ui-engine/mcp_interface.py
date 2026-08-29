"""
MCP Control Interface for the UI Engine.

This module implements the MCP (Model Context Protocol) control surface
that the Agent Core uses to interact with the UI Engine. It exposes
tools for managing UI state, handling events, and rendering.

Architecture Reference:
    - §5 of docs/spec.md (Event routing)
    - §7.1 of agent-native-os-architecture.md (UI Runtime)

Version: 0.1.0
"""

import logging
from typing import Any, Callable, Dict, List, Optional
from runtime import UIRuntime
from models import UIStateTree, PatchOperation

logger = logging.getLogger(__name__)


class MCPControlInterface:
    """
    MCP control surface for the UI Engine.
    
    The UI Engine exposes MCP tools for the agent to:
    - Send UI trees and patches
    - Query UI state
    - Handle user intents
    - Register styles and tokens
    
    Available Tools:
        - ui.render: Render a UI tree or patch
        - ui.patch: Apply patches to existing UI
        - ui.state: Get current UI state
        - ui.event: Handle UI events
        - ui.register_style: Register ASL styles
        - ui.register_component: Register components
        - ui.resolve_intent: Resolve MCP intents
    
    Example:
        >>> mcp = MCPControlInterface()
        >>> result = mcp.handle_tool_call("ui.render", {
        ...     "tree": "stack#root dir=v\\n  text \\"Hello\\""
        ... })
    """
    
    def __init__(self, runtime: Optional[UIRuntime] = None):
        """
        Initialize the MCP Control Interface.
        
        Args:
            runtime: Optional UIRuntime instance (creates new if not provided)
        """
        self.runtime = runtime or UIRuntime()
        
        # Register MCP tools
        self._tools: Dict[str, Callable[[Dict[str, Any]], Dict[str, Any]]] = {
            "ui.render": self._handle_render,
            "ui.patch": self._handle_patch,
            "ui.state": self._handle_state,
            "ui.event": self._handle_event,
            "ui.register_style": self._handle_register_style,
            "ui.register_component": self._handle_register_component,
            "ui.resolve_intent": self._handle_resolve_intent,
            "ui.get_tree": self._handle_get_tree,
            "ui.get_stats": self._handle_get_stats,
        }
        
        logger.info(f"MCPControlInterface initialized with {len(self._tools)} tools")
    
    def handle_tool_call(self, tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle an MCP tool call.
        
        Args:
            tool_name: Name of the tool
            arguments: Tool arguments
            
        Returns:
            Tool result as dictionary
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
    
    def _handle_render(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle ui.render - Render a UI tree or patch.
        
        Args:
            args: Must contain "tree" (AUIL) or "patch" (patch ops)
                  May contain "styles" (ASL)
            
        Returns:
            Rendering result
        """
        tree_source = args.get("tree")
        patch_source = args.get("patch")
        styles_source = args.get("styles")
        
        # Load styles if provided
        if styles_source:
            self.runtime.load_asl(styles_source)
        
        # Render tree or apply patches
        if tree_source:
            root = self.runtime.load_auil(tree_source)
            return {
                "success": True,
                "tree": root.to_dict(),
                "stats": self.runtime.get_stats(),
            }
        elif patch_source:
            success = self.runtime.apply_patch(patch_source)
            return {
                "success": success,
                "stats": self.runtime.get_stats(),
            }
        else:
            return {"error": "Either 'tree' or 'patch' must be provided"}
    
    def _handle_patch(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle ui.patch - Apply patches to existing UI.
        
        Args:
            args: Must contain "patch" (patch operations)
            
        Returns:
            Patch result
        """
        patch_source = args.get("patch")
        if not patch_source:
            return {"error": "'patch' must be provided"}
        
        success = self.runtime.apply_patch(patch_source)
        return {
            "success": success,
            "stats": self.runtime.get_stats(),
        }
    
    def _handle_state(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle ui.state - Get current UI state.
        
        Args:
            args: May contain "node_id" to get specific node state
            
        Returns:
            Current UI state
        """
        node_id = args.get("node_id")
        
        if node_id:
            props = self.runtime.get_node_properties(node_id)
            states = self.runtime.get_active_states(node_id)
            return {
                "node_id": node_id,
                "properties": props,
                "active_states": list(states),
            }
        else:
            tree = self.runtime.get_tree()
            if tree:
                return {
                    "version": tree.version,
                    "node_count": len(tree.nodes),
                    "last_updated": tree.last_updated,
                }
            else:
                return {"error": "No tree loaded"}
    
    def _handle_event(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle ui.event - Handle UI events.
        
        Events are processed locally for real-time states (hover, press, etc.)
        and routed to the agent for semantic actions.
        
        Args:
            args: Must contain "event_type" and "node_id"
                  May contain "data"
            
        Returns:
            Event handling result
        """
        event_type = args.get("event_type")
        node_id = args.get("node_id")
        data = args.get("data", {})
        
        if not event_type or not node_id:
            return {"error": "'event_type' and 'node_id' must be provided"}
        
        # Handle real-time states locally
        real_time_states = {"hover", "press", "release", "focus", "blur", "drag"}
        
        if event_type in real_time_states:
            if event_type in ("hover", "focus", "press", "drag"):
                self.runtime.set_state(node_id, event_type)
            elif event_type in ("release", "blur"):
                self.runtime.clear_state(node_id, event_type)
            
            return {
                "success": True,
                "handled_locally": True,
                "node_id": node_id,
                "state": event_type,
            }
        
        # For semantic actions, route to agent via MCP
        node = self.runtime.find_node(node_id)
        if node:
            # Check for MCP intent
            intent = node.properties.get("on:press") or node.properties.get("on:click")
            if intent:
                resolved = self.runtime.resolve_mcp_intent(str(intent))
                if resolved:
                    return {
                        "success": True,
                        "handled_locally": False,
                        "intent": resolved,
                        "node_id": node_id,
                        "event_type": event_type,
                    }
        
        return {
            "success": True,
            "handled_locally": False,
            "node_id": node_id,
            "event_type": event_type,
        }
    
    def _handle_register_style(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle ui.register_style - Register ASL styles.
        
        Args:
            args: Must contain "styles" (ASL source)
            
        Returns:
            Registration result
        """
        styles_source = args.get("styles")
        if not styles_source:
            return {"error": "'styles' must be provided"}
        
        self.runtime.load_asl(styles_source)
        return {
            "success": True,
            "stats": self.runtime.get_stats(),
        }
    
    def _handle_register_component(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle ui.register_component - Register a component.
        
        Args:
            args: Must contain "name"
                  May contain "parent", "mixins", "slots"
            
        Returns:
            Registration result
        """
        name = args.get("name")
        parent = args.get("parent")
        mixins = args.get("mixins", [])
        slots = args.get("slots", [])
        
        if not name:
            return {"error": "'name' must be provided"}
        
        try:
            from models import SlotDefinition
            slot_defs = [SlotDefinition(name=s) for s in slots]
            
            self.runtime._components.register(
                name=name,
                parent=parent,
                mixins=mixins,
                slots=slot_defs,
            )
            
            return {"success": True, "name": name}
        except Exception as e:
            return {"error": str(e)}
    
    def _handle_resolve_intent(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle ui.resolve_intent - Resolve an MCP intent.
        
        Args:
            args: Must contain "intent" (MCP intent string)
            
        Returns:
            Resolved intent
        """
        intent = args.get("intent")
        if not intent:
            return {"error": "'intent' must be provided"}
        
        resolved = self.runtime.resolve_mcp_intent(intent)
        if resolved:
            return {"success": True, "intent": resolved}
        else:
            return {"error": f"Could not resolve intent: {intent}"}
    
    def _handle_get_tree(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle ui.get_tree - Get the full UI state tree.
        
        Args:
            args: Optional arguments
            
        Returns:
            Full tree representation
        """
        tree = self.runtime.get_tree()
        if tree:
            return {
                "success": True,
                "tree": tree.to_dict(),
            }
        else:
            return {"error": "No tree loaded"}
    
    def _handle_get_stats(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle ui.get_stats - Get UI engine statistics.
        
        Args:
            args: Optional arguments
            
        Returns:
            Statistics
        """
        return {
            "success": True,
            "stats": self.runtime.get_stats(),
        }
    
    def get_available_tools(self) -> List[Dict[str, str]]:
        """
        Get list of available MCP tools.
        
        Returns:
            List of tool dictionaries
        """
        tools = []
        for name in self._tools.keys():
            tools.append({
                "name": name,
                "description": self._get_tool_description(name),
            })
        return tools
    
    def _get_tool_description(self, tool_name: str) -> str:
        """Get description for a tool."""
        descriptions = {
            "ui.render": "Render a UI tree or apply patches",
            "ui.patch": "Apply patches to existing UI",
            "ui.state": "Get current UI state",
            "ui.event": "Handle UI events",
            "ui.register_style": "Register ASL styles",
            "ui.register_component": "Register a UI component",
            "ui.resolve_intent": "Resolve an MCP intent",
            "ui.get_tree": "Get the full UI state tree",
            "ui.get_stats": "Get UI engine statistics",
        }
        return descriptions.get(tool_name, "No description available")
