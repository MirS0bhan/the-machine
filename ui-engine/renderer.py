"""
Renderer Abstraction for Wayland Compositor Integration.

This module provides the abstract renderer interface that connects
the UI Engine to the Wayland compositor (separate project).

Architecture Reference:
    - §7.2 of docs/spec.md (Renderer — external project)
    - §1 of docs/spec.md (Real-time constraint)

Version: 0.1.0
"""

import logging
from abc import ABC, abstractmethod
from typing import Any, Dict, List, Optional, Set, Tuple
from models import UINode, UIStateTree, DesignToken, AdaptiveColor

logger = logging.getLogger(__name__)


class RenderCommand:
    """
    Command to be sent to the compositor renderer.
    
    Attributes:
        type: Command type (create, update, destroy, etc.)
        node_id: Target node ID
        properties: Properties to set
        children: Child node IDs (for create)
    """
    type: str
    node_id: str
    properties: Dict[str, Any] = {}
    children: List[str] = []
    
    def __init__(self, type: str, node_id: str, **kwargs):
        self.type = type
        self.node_id = node_id
        self.properties = kwargs.get("properties", {})
        self.children = kwargs.get("children", [])


class AbstractRenderer(ABC):
    """
    Abstract base class for UI renderers.
    
    The renderer is responsible for translating the UI State Tree
    into visual output. In production, this would be implemented
    by the Wayland compositor project.
    
    Interface:
        - create_surface: Create a new surface/window
        - update_surface: Update surface properties
        - destroy_surface: Destroy a surface
        - commit_batch: Commit a batch of render commands
        - flush: Flush pending commands to compositor
    
    Example:
        >>> class WaylandRenderer(AbstractRenderer):
        ...     def create_surface(self, surface_id, props):
        ...         # Create Wayland surface
        ...         pass
    """
    
    @abstractmethod
    def create_surface(
        self,
        surface_id: str,
        properties: Dict[str, Any],
    ) -> bool:
        """
        Create a new surface/window.
        
        Args:
            surface_id: Unique surface identifier
            properties: Surface properties (size, position, etc.)
            
        Returns:
            True if successful
        """
        pass
    
    @abstractmethod
    def update_surface(
        self,
        surface_id: str,
        properties: Dict[str, Any],
    ) -> bool:
        """
        Update surface properties.
        
        Args:
            surface_id: Surface identifier
            properties: Properties to update
            
        Returns:
            True if successful
        """
        pass
    
    @abstractmethod
    def destroy_surface(self, surface_id: str) -> bool:
        """
        Destroy a surface.
        
        Args:
            surface_id: Surface identifier
            
        Returns:
            True if successful
        """
        pass
    
    @abstractmethod
    def commit_batch(self, commands: List[RenderCommand]) -> bool:
        """
        Commit a batch of render commands.
        
        Args:
            commands: List of render commands
            
        Returns:
            True if all commands committed successfully
        """
        pass
    
    @abstractmethod
    def flush(self) -> bool:
        """
        Flush pending commands to compositor.
        
        Returns:
            True if flush successful
        """
        pass
    
    @abstractmethod
    def get_surface_state(self, surface_id: str) -> Optional[Dict[str, Any]]:
        """
        Get current state of a surface.
        
        Args:
            surface_id: Surface identifier
            
        Returns:
            Surface state dictionary or None
        """
        pass


class MockRenderer(AbstractRenderer):
    """
    Mock renderer for testing.
    
    Stores render commands in memory without actually rendering.
    
    Example:
        >>> renderer = MockRenderer()
        >>> renderer.create_surface("root", {"width": 800, "height": 600})
        >>> commands = renderer.get_commands()
    """
    
    def __init__(self):
        """Initialize the mock renderer."""
        self._surfaces: Dict[str, Dict[str, Any]] = {}
        self._commands: List[RenderCommand] = []
    
    def create_surface(
        self,
        surface_id: str,
        properties: Dict[str, Any],
    ) -> bool:
        """Create a surface in the mock renderer."""
        self._surfaces[surface_id] = properties
        self._commands.append(RenderCommand(
            type="create",
            node_id=surface_id,
            properties=properties,
        ))
        return True
    
    def update_surface(
        self,
        surface_id: str,
        properties: Dict[str, Any],
    ) -> bool:
        """Update a surface in the mock renderer."""
        if surface_id in self._surfaces:
            self._surfaces[surface_id].update(properties)
            self._commands.append(RenderCommand(
                type="update",
                node_id=surface_id,
                properties=properties,
            ))
            return True
        return False
    
    def destroy_surface(self, surface_id: str) -> bool:
        """Destroy a surface in the mock renderer."""
        if surface_id in self._surfaces:
            del self._surfaces[surface_id]
            self._commands.append(RenderCommand(
                type="destroy",
                node_id=surface_id,
            ))
            return True
        return False
    
    def commit_batch(self, commands: List[RenderCommand]) -> bool:
        """Commit commands to the mock renderer."""
        for cmd in commands:
            if cmd.type == "create":
                self._surfaces[cmd.node_id] = cmd.properties
            elif cmd.type == "update":
                if cmd.node_id in self._surfaces:
                    self._surfaces[cmd.node_id].update(cmd.properties)
            elif cmd.type == "destroy":
                if cmd.node_id in self._surfaces:
                    del self._surfaces[cmd.node_id]
        
        self._commands.extend(commands)
        return True
    
    def flush(self) -> bool:
        """Flush (no-op for mock)."""
        return True
    
    def get_surface_state(self, surface_id: str) -> Optional[Dict[str, Any]]:
        """Get surface state from mock renderer."""
        return self._surfaces.get(surface_id)
    
    def get_commands(self) -> List[RenderCommand]:
        """Get all recorded commands."""
        return self._commands.copy()
    
    def clear_commands(self) -> None:
        """Clear recorded commands."""
        self._commands.clear()
    
    def get_surfaces(self) -> Dict[str, Dict[str, Any]]:
        """Get all surfaces."""
        return self._surfaces.copy()


class TreeRenderer:
    """
    Renders a UI State Tree using an AbstractRenderer.
    
    This class translates the UI State Tree into render commands
    and sends them to the underlying renderer.
    
    Example:
        >>> renderer = MockRenderer()
        >>> tree_renderer = TreeRenderer(renderer)
        >>> tree_renderer.render(tree)
    """
    
    def __init__(self, renderer: AbstractRenderer):
        """
        Initialize the tree renderer.
        
        Args:
            renderer: Abstract renderer implementation
        """
        self._renderer = renderer
        self._rendered_nodes: Set[str] = set()
    
    def render(self, tree: UIStateTree) -> bool:
        """
        Render the full UI State Tree.
        
        Args:
            tree: UI State Tree to render
            
        Returns:
            True if successful
        """
        commands = self._generate_commands(tree.root)
        
        if commands:
            success = self._renderer.commit_batch(commands)
            if success:
                self._rendered_nodes = {cmd.node_id for cmd in commands}
            return success
        
        return True
    
    def update(self, tree: UIStateTree) -> bool:
        """
        Update changed nodes in the tree.
        
        Args:
            tree: UI State Tree with changes
            
        Returns:
            True if successful
        """
        commands = self._generate_update_commands(tree.root)
        
        if commands:
            return self._renderer.commit_batch(commands)
        
        return True
    
    def _generate_commands(self, node: UINode) -> List[RenderCommand]:
        """
        Generate render commands for a node and its children.
        
        Args:
            node: Root node to generate commands for
            
        Returns:
            List of render commands
        """
        commands = []
        
        # Create command for this node
        props = self._translate_properties(node)
        commands.append(RenderCommand(
            type="create",
            node_id=node.id or node.tag,
            properties=props,
            children=[c.id or c.tag for c in node.children],
        ))
        
        # Recursively generate commands for children
        for child in node.children:
            commands.extend(self._generate_commands(child))
        
        return commands
    
    def _generate_update_commands(self, node: UINode) -> List[RenderCommand]:
        """
        Generate update commands for changed nodes.
        
        Args:
            node: Root node to generate update commands for
            
        Returns:
            List of update commands
        """
        commands = []
        
        # Generate update command for this node
        props = self._translate_properties(node)
        commands.append(RenderCommand(
            type="update",
            node_id=node.id or node.tag,
            properties=props,
        ))
        
        # Recursively generate commands for children
        for child in node.children:
            commands.extend(self._generate_update_commands(child))
        
        return commands
    
    def _translate_properties(self, node: UINode) -> Dict[str, Any]:
        """
        Translate AUIL properties to renderer properties.
        
        Args:
            node: UINode to translate
            
        Returns:
            Renderer-compatible properties
        """
        props = dict(node.properties)
        
        # Add node metadata
        props["tag"] = node.tag
        if node.id:
            props["id"] = node.id
        if node.mixins:
            props["mixins"] = node.mixins
        if node.text_content and "text" not in props:
            props["text"] = node.text_content
        
        return props
