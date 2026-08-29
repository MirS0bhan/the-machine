"""
Component Registry and Inheritance System.

This module manages UI components with inheritance, mixins, and slot-based
composition. Components are structural templates that the agent can reuse.

Architecture Reference:
    - §2.3 of docs/spec.md (Components)
    - §2.4 of docs/spec.md (Component inheritance & slots)

Version: 0.1.0
"""

import logging
from typing import Any, Dict, List, Optional, Set
from models import (
    UINode,
    ComponentDefinition,
    SlotDefinition,
)

logger = logging.getLogger(__name__)


# Default built-in components
DEFAULT_COMPONENTS = {
    "Surface": ComponentDefinition(
        name="Surface",
        mixins=["Surface"],
    ),
    "Card": ComponentDefinition(
        name="Card",
        parent="Surface",
        mixins=["Surface", "Hoverable"],
    ),
    "ListRow": ComponentDefinition(
        name="ListRow",
        parent="Surface",
        mixins=["Surface", "Hoverable", "Pressable"],
    ),
    "PrimaryButton": ComponentDefinition(
        name="PrimaryButton",
        parent="Surface",
        mixins=["Surface", "Hoverable", "Pressable"],
    ),
    "IconBtn": ComponentDefinition(
        name="IconBtn",
        parent="Surface",
        mixins=["Surface", "Hoverable", "Pressable"],
    ),
    "Field": ComponentDefinition(
        name="Field",
        mixins=["Surface"],
        slots=[SlotDefinition(name="label"), SlotDefinition(name="input")],
    ),
    "MediaPlayer": ComponentDefinition(
        name="MediaPlayer",
        mixins=["Surface"],
        slots=[SlotDefinition(name="video"), SlotDefinition(name="controls")],
    ),
    "Chart": ComponentDefinition(
        name="Chart",
        mixins=["Surface"],
        slots=[SlotDefinition(name="data")],
    ),
}


class ComponentRegistry:
    """
    Registry for UI components.
    
    Manages component definitions, inheritance resolution, and
    mixin composition.
    
    Example:
        >>> registry = ComponentRegistry()
        >>> registry.register("VideoCard", parent="Card", mixins=["VideoPlayer"])
        >>> node = registry.create_instance("VideoCard", id="player1")
    """
    
    def __init__(self):
        """Initialize with default components."""
        self._components: Dict[str, ComponentDefinition] = {}
        
        # Register default components
        for name, comp in DEFAULT_COMPONENTS.items():
            self._components[name] = comp
        
        logger.info(f"ComponentRegistry initialized with {len(self._components)} default components")
    
    def register(
        self,
        name: str,
        parent: Optional[str] = None,
        mixins: Optional[List[str]] = None,
        slots: Optional[List[SlotDefinition]] = None,
    ) -> ComponentDefinition:
        """
        Register a new component.
        
        Args:
            name: Component name
            parent: Parent component name (for inheritance)
            mixins: Style mixin names
            slots: Slot definitions
            
        Returns:
            The registered ComponentDefinition
            
        Raises:
            ValueError: If parent component doesn't exist
        """
        if parent and parent not in self._components:
            raise ValueError(f"Parent component '{parent}' not found")
        
        # Resolve mixins from parent
        all_mixins = list(mixins or [])
        if parent:
            parent_comp = self._components[parent]
            # Parent mixins come first (child mixins override)
            all_mixins = parent_comp.mixins + all_mixins
        
        # Resolve slots from parent
        all_slots = list(slots or [])
        if parent:
            parent_comp = self._components[parent]
            all_slots = parent_comp.slots + all_slots
        
        comp = ComponentDefinition(
            name=name,
            parent=parent,
            mixins=all_mixins,
            slots=all_slots,
        )
        
        self._components[name] = comp
        logger.info(f"Registered component '{name}' (parent={parent}, mixins={all_mixins})")
        
        return comp
    
    def get(self, name: str) -> Optional[ComponentDefinition]:
        """
        Get a component definition.
        
        Args:
            name: Component name
            
        Returns:
            ComponentDefinition if found, None otherwise
        """
        return self._components.get(name)
    
    def resolve_mixins(self, name: str) -> List[str]:
        """
        Resolve all mixins for a component (including inherited).
        
        Args:
            name: Component name
            
        Returns:
            List of mixin names
        """
        comp = self._components.get(name)
        if not comp:
            return []
        
        # Mixins are already resolved in registration
        return comp.mixins
    
    def resolve_slots(self, name: str) -> List[SlotDefinition]:
        """
        Resolve all slots for a component (including inherited).
        
        Args:
            name: Component name
            
        Returns:
            List of slot definitions
        """
        comp = self._components.get(name)
        if not comp:
            return []
        
        return comp.slots
    
    def create_instance(
        self,
        name: str,
        id: Optional[str] = None,
        properties: Optional[Dict[str, Any]] = None,
        children: Optional[List[UINode]] = None,
    ) -> UINode:
        """
        Create an instance of a component.
        
        Args:
            name: Component name
            id: Optional node ID
            properties: Node properties
            children: Child nodes
            
        Returns:
            UINode instance
            
        Raises:
            ValueError: If component doesn't exist
        """
        comp = self._components.get(name)
        if not comp:
            raise ValueError(f"Component '{name}' not found")
        
        node = UINode(
            tag=name,
            id=id,
            mixins=comp.mixins.copy(),
            properties=properties or {},
            children=children or [],
        )
        
        return node
    
    def list_components(self) -> List[Dict[str, Any]]:
        """
        List all registered components.
        
        Returns:
            List of component info dictionaries
        """
        result = []
        for name, comp in self._components.items():
            info: Dict[str, Any] = {
                "name": name,
                "mixins": comp.mixins,
                "slots": [s.name for s in comp.slots],
            }
            if comp.parent:
                info["parent"] = comp.parent
            result.append(info)
        return result
    
    def validate_slots(
        self,
        name: str,
        provided_slots: Set[str],
    ) -> List[str]:
        """
        Validate that all required slots are provided.
        
        Args:
            name: Component name
            provided_slots: Set of provided slot names
            
        Returns:
            List of missing required slot names
        """
        comp = self._components.get(name)
        if not comp:
            return []
        
        missing = []
        for slot in comp.slots:
            if slot.required and slot.name not in provided_slots:
                missing.append(slot.name)
        
        return missing
