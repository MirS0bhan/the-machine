"""
UI Runtime for the AUIL/ASL UI Engine.

This module implements the core UI Runtime that manages the UI State Tree,
handles patching, and coordinates with the renderer.

Architecture Reference:
    - §1 of docs/spec.md (The core architectural constraint)
    - §6 of docs/spec.md (Motion as state, not keyframes)
    - §7 of docs/spec.md (Component inheritance & slots)

Version: 0.1.0
"""

import logging
import time
from typing import Any, Callable, Dict, List, Optional, Set
from models import (
    UINode,
    UIStateTree,
    PatchOperation,
    PatchOp,
    StyleMixin,
    StateTransition,
    DesignToken,
    MotionCurve,
    EventType,
)
from auil_parser import parse_auil, AUILParser
from asl_parser import parse_asl, ASLParser
from components import ComponentRegistry
from patch_protocol import PatchParser, PatchApplicator, parse_patches

logger = logging.getLogger(__name__)


class UIRuntime:
    """
    Core UI Runtime.
    
    The UIRuntime is the central coordinator that:
    - Manages the UI State Tree
    - Applies patches from the agent
    - Coordinates with the renderer for display
    - Handles real-time state transitions (hover, press, etc.)
    - Routes semantic actions to the agent via MCP
    
    Design Principles:
        1. Real-time stays out of the agent's path
        2. Structure and style are separate languages
        3. Patch-native, not render-native
        4. Components support mixin composition
    
    Example:
        >>> runtime = UIRuntime()
        >>> runtime.load_auil('''
        ... stack#root dir=v
        ...   text(role=title) "Hello World"
        ...   button#ok label=OK on:press=mcp:app.confirm
        ... ''')
        >>> runtime.apply_patch("~ok(color=accent)")
    """
    
    def __init__(self):
        """Initialize the UI Runtime."""
        self._tree: Optional[UIStateTree] = None
        self._components = ComponentRegistry()
        self._styles: Dict[str, StyleMixin] = {}
        self._tokens: Dict[str, DesignToken] = {}
        self._motions: Dict[str, MotionCurve] = {}
        
        # Parsers
        self._auil_parser = AUILParser()
        self._asl_parser = ASLParser()
        self._patch_parser = PatchParser()
        self._patch_applicator = PatchApplicator()
        
        # State management
        self._active_states: Dict[str, Set[str]] = {}  # node_id -> set of states
        self._event_handlers: Dict[str, Callable] = {}
        
        # Renderer callback
        self._render_callback: Optional[Callable[[UIStateTree], None]] = None
        
        logger.info("UIRuntime initialized")
    
    def set_render_callback(self, callback: Callable[[UIStateTree], None]) -> None:
        """
        Set the render callback.
        
        Args:
            callback: Function to call when the tree needs rendering
        """
        self._render_callback = callback
        logger.debug("Render callback set")
    
    def load_auil(self, source: str) -> UINode:
        """
        Load AUIL source and create the initial UI tree.
        
        Args:
            source: AUIL source code
            
        Returns:
            Root node of the parsed tree
        """
        root = parse_auil(source)
        
        # Create state tree
        self._tree = UIStateTree(root=root)
        
        # Resolve component references
        self._resolve_components(root)
        
        logger.info(f"Loaded AUIL tree: {len(self._tree.nodes)} nodes")
        return root
    
    def load_asl(self, source: str) -> None:
        """
        Load ASL source and apply styles.
        
        Args:
            source: ASL source code
        """
        result = parse_asl(source)
        
        self._tokens.update(result.get("tokens", {}))
        self._styles.update(result.get("styles", {}))
        self._motions.update(result.get("motions", {}))
        
        logger.info(
            f"Loaded ASL: {len(self._tokens)} tokens, "
            f"{len(self._styles)} styles, {len(self._motions)} motions"
        )
    
    def load_styles(self, source: str) -> None:
        """
        Load ASL styles (alias for load_asl).
        
        Args:
            source: ASL source code
        """
        self.load_asl(source)
    
    def apply_patch(self, patch_source: str) -> bool:
        """
        Apply a patch operation string.
        
        Args:
            patch_source: Patch operation string
            
        Returns:
            True if patch applied successfully
        """
        if not self._tree:
            logger.warning("No tree loaded, cannot apply patch")
            return False
        
        ops = parse_patches(patch_source)
        if not ops:
            return False
        
        success = self._patch_applicator.apply(self._tree, ops)
        
        if success:
            self._trigger_render()
        
        return success
    
    def apply_patches(self, patches: List[PatchOperation]) -> bool:
        """
        Apply a list of patch operations.
        
        Args:
            patches: List of PatchOperations
            
        Returns:
            True if all patches applied successfully
        """
        if not self._tree:
            logger.warning("No tree loaded, cannot apply patches")
            return False
        
        success = self._patch_applicator.apply(self._tree, patches)
        
        if success:
            self._trigger_render()
        
        return success
    
    def find_node(self, node_id: str) -> Optional[UINode]:
        """
        Find a node by ID.
        
        Args:
            node_id: Node ID
            
        Returns:
            UINode if found, None otherwise
        """
        if not self._tree:
            return None
        return self._tree.find_node(node_id)
    
    def update_node(self, node_id: str, properties: Dict[str, Any]) -> bool:
        """
        Update properties on a node.
        
        Args:
            node_id: Node ID
            properties: Properties to update
            
        Returns:
            True if successful
        """
        node = self.find_node(node_id)
        if not node:
            return False
        
        node.properties.update(properties)
        self._trigger_render()
        return True
    
    def get_node_properties(self, node_id: str) -> Dict[str, Any]:
        """
        Get all properties for a node (including from mixins and tokens).
        
        Args:
            node_id: Node ID
            
        Returns:
            Combined properties dictionary
        """
        node = self.find_node(node_id)
        if not node:
            return {}
        
        # Start with node properties
        props = dict(node.properties)
        
        # Apply mixin properties
        for mixin_name in node.mixins:
            style = self._styles.get(mixin_name)
            if style:
                # Apply base properties (lower priority)
                for key, value in style.properties.items():
                    if key not in props:
                        props[key] = value
        
        # Resolve token references
        props = self._resolve_tokens(props)
        
        return props
    
    def set_state(self, node_id: str, state: str) -> None:
        """
        Set a state on a node (e.g., "hover", "press").
        
        This is called by the renderer when real-time events occur.
        The state transition is resolved locally without MCP round-trip.
        
        Args:
            node_id: Node ID
            state: State name
        """
        if node_id not in self._active_states:
            self._active_states[node_id] = set()
        
        self._active_states[node_id].add(state)
        
        # Apply state transitions
        self._apply_state_transitions(node_id)
    
    def clear_state(self, node_id: str, state: str) -> None:
        """
        Clear a state on a node.
        
        Args:
            node_id: Node ID
            state: State name
        """
        if node_id in self._active_states:
            self._active_states[node_id].discard(state)
            self._apply_state_transitions(node_id)
    
    def get_active_states(self, node_id: str) -> Set[str]:
        """
        Get active states for a node.
        
        Args:
            node_id: Node ID
            
        Returns:
            Set of active state names
        """
        return self._active_states.get(node_id, set())
    
    def _apply_state_transitions(self, node_id: str) -> None:
        """
        Apply state transitions for a node.
        
        Args:
            node_id: Node ID
        """
        node = self.find_node(node_id)
        if not node:
            return
        
        active_states = self._active_states.get(node_id, set())
        
        # Apply transitions from mixins
        for mixin_name in node.mixins:
            style = self._styles.get(mixin_name)
            if style:
                for state_name, transition in style.transitions.items():
                    if state_name in active_states:
                        # Apply transition properties
                        node.properties.update(transition.properties)
        
        self._trigger_render()
    
    def resolve_mcp_intent(self, intent: str) -> Optional[Dict[str, Any]]:
        """
        Resolve an MCP intent for a node action.
        
        Args:
            intent: MCP intent string
            
        Returns:
            Intent dictionary or None
        """
        # Parse intent (e.g., "mcp:video_player.toggle")
        if intent.startswith("mcp:"):
            path = intent[4:]
            parts = path.split(".")
            
            if len(parts) >= 2:
                return {
                    "handler": parts[0],
                    "action": parts[1],
                    "args": parts[2:] if len(parts) > 2 else [],
                }
        
        return None
    
    def _resolve_components(self, node: UINode) -> None:
        """
        Resolve component references in the tree.
        
        Args:
            node: Root node to resolve
        """
        # Check if this node is a registered component
        comp = self._components.get(node.tag)
        if comp:
            # Apply component's mixins
            for mixin in comp.mixins:
                if mixin not in node.mixins:
                    node.mixins.append(mixin)
        
        # Recursively resolve children
        for child in node.children:
            self._resolve_components(child)
    
    def _resolve_tokens(self, props: Dict[str, Any]) -> Dict[str, Any]:
        """
        Resolve token references in properties.
        
        Args:
            props: Properties dictionary
            
        Returns:
            Resolved properties
        """
        resolved = {}
        
        for key, value in props.items():
            if isinstance(value, dict) and value.get("type") == "token_ref":
                token_name = value.get("name", "")
                token = self._tokens.get(token_name)
                if token:
                    # For now, use the value directly
                    # In production, would resolve adaptive colors based on theme
                    resolved[key] = token.value
                else:
                    resolved[key] = value
            else:
                resolved[key] = value
        
        return resolved
    
    def _trigger_render(self) -> None:
        """Trigger a render update."""
        if self._render_callback and self._tree:
            self._render_callback(self._tree)
    
    def get_tree(self) -> Optional[UIStateTree]:
        """Get the current UI State Tree."""
        return self._tree
    
    def get_stats(self) -> Dict[str, Any]:
        """
        Get runtime statistics.
        
        Returns:
            Statistics dictionary
        """
        return {
            "nodes": len(self._tree.nodes) if self._tree else 0,
            "version": self._tree.version if self._tree else 0,
            "styles": len(self._styles),
            "tokens": len(self._tokens),
            "motions": len(self._motions),
            "components": len(self._components.list_components()),
            "patches_applied": self._patch_applicator.get_stats()["applied"],
        }
