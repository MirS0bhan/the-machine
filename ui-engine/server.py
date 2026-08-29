"""
UI Engine Server - Main Entry Point.

This module provides the main UIEngine class that ties together
all components of the AUIL/ASL UI Engine.

Architecture Reference:
    - §7.1 of agent-native-os-architecture.md (UI Runtime)
    - docs/spec.md (AUIL + ASL specification)

Version: 0.1.0
"""

import logging
from typing import Any, Callable, Dict, List, Optional
from runtime import UIRuntime
from mcp_interface import MCPControlInterface
from renderer import AbstractRenderer, MockRenderer, TreeRenderer
from models import UIStateTree

logger = logging.getLogger(__name__)


class UIEngine:
    """
    UI Engine for the Agent-Native OS.
    
    The UI Engine provides a complete UI rendering pipeline:
    - AUIL for structure
    - ASL for styles and motion
    - Patch protocol for updates
    - MCP interface for agent interaction
    - Renderer abstraction for Wayland compositor
    
    Design Principles:
        1. Cheap for an LLM to emit and patch
        2. Patch-native, not render-native
        3. Structure and style are separate languages
        4. Real-time stays out of the agent's path
        5. Composition over regeneration
    
    Example:
        >>> engine = UIEngine()
        >>> engine.render('''
        ... stack#root dir=v gap=m
        ...   text(role=title) "Hello World"
        ...   button#ok label=OK on:press=mcp:app.confirm
        ... ''')
    """
    
    def __init__(self, renderer: Optional[AbstractRenderer] = None):
        """
        Initialize the UI Engine.
        
        Args:
            renderer: Optional renderer (uses MockRenderer if not provided)
        """
        self._runtime = UIRuntime()
        self._mcp = MCPControlInterface(self._runtime)
        self._renderer = renderer or MockRenderer()
        self._tree_renderer = TreeRenderer(self._renderer)
        
        # Connect render callback
        self._runtime.set_render_callback(self._on_tree_update)
        
        logger.info("UIEngine initialized")
    
    def render(self, source: str, styles: Optional[str] = None) -> Dict[str, Any]:
        """
        Render AUIL source with optional ASL styles.
        
        Args:
            source: AUIL source code
            styles: Optional ASL styles
            
        Returns:
            Render result
        """
        # Load styles if provided
        if styles:
            self._runtime.load_asl(styles)
        
        # Load and render tree
        root = self._runtime.load_auil(source)
        tree = self._runtime.get_tree()
        
        if tree:
            self._tree_renderer.render(tree)
        
        return {
            "success": True,
            "tree": root.to_dict(),
            "stats": self._runtime.get_stats(),
        }
    
    def patch(self, patch_source: str) -> Dict[str, Any]:
        """
        Apply patches to the current UI.
        
        Args:
            patch_source: Patch operations
            
        Returns:
            Patch result
        """
        success = self._runtime.apply_patch(patch_source)
        return {
            "success": success,
            "stats": self._runtime.get_stats(),
        }
    
    def handle_mcp_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle an MCP tool call.
        
        Args:
            tool_name: MCP tool name
            arguments: Tool arguments
            
        Returns:
            Tool result
        """
        return self._mcp.handle_tool_call(tool_name, arguments)
    
    def get_tools(self) -> List[Dict[str, str]]:
        """
        Get available MCP tools.
        
        Returns:
            List of tool definitions
        """
        return self._mcp.get_available_tools()
    
    def _on_tree_update(self, tree: UIStateTree) -> None:
        """
        Callback for tree updates.
        
        Args:
            tree: Updated UI State Tree
        """
        self._tree_renderer.update(tree)
    
    def get_renderer(self) -> AbstractRenderer:
        """Get the current renderer."""
        return self._renderer
    
    def set_renderer(self, renderer: AbstractRenderer) -> None:
        """
        Set a new renderer.
        
        Args:
            renderer: New renderer implementation
        """
        self._renderer = renderer
        self._tree_renderer = TreeRenderer(renderer)
        logger.info("Renderer updated")
    
    def get_stats(self) -> Dict[str, Any]:
        """
        Get engine statistics.
        
        Returns:
            Statistics dictionary
        """
        return self._runtime.get_stats()


def create_engine(renderer: Optional[AbstractRenderer] = None) -> UIEngine:
    """
    Create and return a UI Engine instance.
    
    Args:
        renderer: Optional renderer implementation
        
    Returns:
        Configured UIEngine instance
        
    Example:
        >>> from ui_engine import create_engine
        >>> engine = create_engine()
        >>> engine.render('stack dir=v\\n  text "Hello"')
    """
    return UIEngine(renderer=renderer)
