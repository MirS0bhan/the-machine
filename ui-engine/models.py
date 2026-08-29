"""
Core data models for the AUIL/ASL UI Engine.

This module defines the fundamental data structures for the Agent UI Layout
(AUIL) and Agent Style Language (ASL), including UI nodes, style mixins,
design tokens, and the patch protocol.

Architecture Reference:
    - §7.1 of agent-native-os-architecture.md (Exact declarative UI schema)
    - docs/spec.md (AUIL + ASL specification)

Version: 0.1.0
"""

from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Any, Dict, List, Optional, Set, Tuple, Union
import hashlib
import logging
import time
import uuid

logger = logging.getLogger(__name__)


# =============================================================================
# Enums
# =============================================================================

class PrimitiveTag(Enum):
    """Fixed primitive tags for AUIL nodes."""
    STACK = "stack"
    GRID = "grid"
    TEXT = "text"
    FIELD = "field"
    BUTTON = "button"
    LIST = "list"
    MEDIA = "media"
    CHART = "chart"
    ICON = "icon"
    SLIDER = "slider"
    TOGGLE = "toggle"


class TextRole(Enum):
    """Text roles for accessibility and type scale."""
    TITLE = "title"
    BODY = "body"
    CAPTION = "caption"
    LABEL = "label"


class MediaType(Enum):
    """Media types for the media primitive."""
    VIDEO = "video"
    AUDIO = "audio"
    IMAGE = "image"


class ChartType(Enum):
    """Chart types for the chart primitive."""
    LINE = "line"
    BAR = "bar"
    PIE = "pie"


class LayoutDirection(Enum):
    """Layout direction for stack and other containers."""
    HORIZONTAL = "h"
    VERTICAL = "v"


class PatchOp(Enum):
    """Patch operation types."""
    UPDATE = "~"      # Update props in place
    INSERT = "+"      # Insert node
    REMOVE = "-"      # Remove node
    REPLACE = "!"     # Replace subtree
    MOVE = "@"        # Move/reorder node


class EventType(Enum):
    """Event types for ASL state transitions."""
    HOVER = "hover"
    PRESS = "press"
    RELEASE = "release"
    FOCUS = "focus"
    BLUR = "blur"
    DRAG = "drag"
    CHANGE = "change"
    LOADING = "loading"
    ERROR = "error"
    IDLE = "idle"


class ReferenceType(Enum):
    """Reference sigils for property values."""
    LAMBDA = "$lambda:"    # Binds to a running lambda's output
    MCP = "mcp:"           # Names an MCP intent to invoke
    STATE = "@"            # Binds to State Store data path


# =============================================================================
# Design Tokens
# =============================================================================

@dataclass
class AdaptiveColor:
    """
    Adaptive color that resolves based on system theme.
    
    Attributes:
        light: Color value for light theme
        dark: Color value for dark theme
    """
    light: str
    dark: str
    
    def resolve(self, is_dark: bool = False) -> str:
        """Resolve color based on theme."""
        return self.dark if is_dark else self.light


@dataclass
class DesignToken:
    """
    Design token for consistent styling.
    
    Tokens are the only legal way to specify color, radius, spacing,
    elevation, and motion — no raw hex/px in AUIL or ASL.
    
    Attributes:
        name: Token name (e.g., "surface.primary")
        value: Token value (can be AdaptiveColor, string, or number)
        vibrancy: Vibrancy level for compositor backdrop blur
    """
    name: str
    value: Union[AdaptiveColor, str, int, float]
    vibrancy: Optional[str] = None


@dataclass
class MotionCurve:
    """
    Motion curve for animations.
    
    Attributes:
        name: Motion name (e.g., "snappy", "gentle")
        type: Motion type ("spring" or "duration")
        stiffness: Spring stiffness (for spring type)
        damping: Spring damping (for spring type)
        duration_ms: Duration in milliseconds (for duration type)
        easing: Easing function (for duration type)
    """
    name: str
    type: str  # "spring" or "duration"
    stiffness: Optional[float] = None
    damping: Optional[float] = None
    duration_ms: Optional[float] = None
    easing: Optional[str] = None


@dataclass
class Scale:
    """
    Scale for consistent sizing.
    
    Attributes:
        name: Scale name (e.g., "radius", "space", "elev")
        values: Mapping of scale names to values
    """
    name: str
    values: Dict[str, Union[int, float, str]]


# =============================================================================
# Style System (ASL)
# =============================================================================

@dataclass
class StateTransition:
    """
    State transition for ASL mixins.
    
    Attributes:
        state: Target state (e.g., "hover", "press")
        properties: Properties to apply in this state
        motion: Motion curve to use for transition
    """
    state: str
    properties: Dict[str, Any] = field(default_factory=dict)
    motion: Optional[str] = None


@dataclass
class StyleMixin:
    """
    ASL style mixin — pure key→state-transition maps.
    
    Mixins are pure key→state-transition maps. A node applying
    `.Surface.Hoverable.Pressable` gets all three behaviors composed;
    conflicts resolve last-applied-wins, left to right.
    
    Attributes:
        name: Mixin name (e.g., "Surface", "Hoverable")
        properties: Base properties
        transitions: State transitions
    """
    name: str
    properties: Dict[str, Any] = field(default_factory=dict)
    transitions: Dict[str, StateTransition] = field(default_factory=dict)


# =============================================================================
# AUIL Node System
# =============================================================================

@dataclass
class Reference:
    """
    Reference to external data or intent.
    
    Attributes:
        type: Reference type (LAMBDA, MCP, STATE)
        path: Reference path (e.g., "video_player.stream", "player.position")
    """
    type: ReferenceType
    path: str
    
    @classmethod
    def parse(cls, value: str) -> "Reference":
        """Parse a reference string."""
        if value.startswith("$lambda:"):
            return cls(type=ReferenceType.LAMBDA, path=value[8:])
        elif value.startswith("mcp:"):
            return cls(type=ReferenceType.MCP, path=value[4:])
        elif value.startswith("@"):
            return cls(type=ReferenceType.STATE, path=value[1:])
        else:
            raise ValueError(f"Invalid reference: {value}")


@dataclass
class Property:
    """
    Node property.
    
    Attributes:
        key: Property key
        value: Property value (can be string, number, bool, or Reference)
    """
    key: str
    value: Union[str, int, float, bool, Reference]
    
    @classmethod
    def parse(cls, key: str, value: str) -> "Property":
        """Parse a property string."""
        # Check for references
        if value.startswith(("$lambda:", "mcp:", "@")):
            return cls(key=key, value=Reference.parse(value))
        
        # Try to parse as number
        try:
            if "." in value:
                return cls(key=key, value=float(value))
            else:
                return cls(key=key, value=int(value))
        except ValueError:
            pass
        
        # Try to parse as bool
        if value.lower() in ("true", "yes"):
            return cls(key=key, value=True)
        elif value.lower() in ("false", "no"):
            return cls(key=key, value=False)
        
        # String value
        return cls(key=key, value=value)


@dataclass
class UINode:
    """
    UI node in the AUIL tree.
    
    Each node has a tag, optional id, mixins, properties, and children.
    The tree is the fundamental data structure for the UI.
    
    Attributes:
        tag: Node tag (primitive or component name)
        id: Optional stable ID (auto-generated if omitted)
        mixins: List of ASL style mixin names
        properties: Node properties
        children: Child nodes
        text_content: Optional text content (shorthand for single text child)
        slots: Slot definitions (for components)
        parent: Parent node reference
    """
    tag: str
    id: Optional[str] = None
    mixins: List[str] = field(default_factory=list)
    properties: Dict[str, Any] = field(default_factory=dict)
    children: List["UINode"] = field(default_factory=list)
    text_content: Optional[str] = None
    slots: Dict[str, bool] = field(default_factory=dict)  # name -> required
    parent: Optional["UINode"] = None
    
    def __post_init__(self):
        """Auto-generate ID if not provided."""
        if self.id is None:
            self.id = f"{self.tag}-{uuid.uuid4().hex[:8]}"
    
    @property
    def path(self) -> str:
        """Get the node's path in the tree."""
        if self.parent is None:
            return self.id or self.tag
        parent_path = self.parent.path
        index = self.parent.children.index(self) if self in self.parent.children else 0
        return f"{parent_path}/{index}"
    
    def find_by_id(self, node_id: str) -> Optional["UINode"]:
        """Find a node by ID in the subtree."""
        if self.id == node_id:
            return self
        for child in self.children:
            found = child.find_by_id(node_id)
            if found:
                return found
        return None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary representation."""
        result: Dict[str, Any] = {
            "tag": self.tag,
            "id": self.id,
        }
        if self.mixins:
            result["mixins"] = self.mixins
        if self.properties:
            result["properties"] = self.properties
        if self.text_content:
            result["text_content"] = self.text_content
        if self.children:
            result["children"] = [c.to_dict() for c in self.children]
        return result


# =============================================================================
# Patch Protocol
# =============================================================================

@dataclass
class PatchOperation:
    """
    Patch operation for UI updates.
    
    The agent's steady-state output is never a full tree. It's one or more
    patch ops against existing ids.
    
    Attributes:
        op: Operation type
        target: Target node ID or anchor
        node: Node to insert/replace (for INSERT, REPLACE)
        properties: Properties to update (for UPDATE)
        source: Source ID (for MOVE)
        destination: Destination path (for MOVE)
    """
    op: PatchOp
    target: str
    node: Optional[UINode] = None
    properties: Optional[Dict[str, Any]] = None
    source: Optional[str] = None
    destination: Optional[str] = None
    
    @classmethod
    def parse(cls, line: str) -> "PatchOperation":
        """Parse a patch operation line."""
        line = line.strip()
        if not line:
            raise ValueError("Empty patch line")
        
        op_char = line[0]
        
        if op_char == "~":
            # Update: ~id(props)
            rest = line[1:]
            paren_idx = rest.find("(")
            if paren_idx == -1:
                # Simple update without props
                return cls(op=PatchOp.UPDATE, target=rest)
            target = rest[:paren_idx]
            props_str = rest[paren_idx + 1:-1]  # Remove trailing )
            properties = cls._parse_properties(props_str)
            return cls(op=PatchOp.UPDATE, target=target, properties=properties)
        
        elif op_char == "+":
            # Insert: +anchor: node
            rest = line[1:]
            colon_idx = rest.find(":")
            if colon_idx == -1:
                raise ValueError(f"Invalid insert syntax: {line}")
            anchor = rest[:colon_idx].strip()
            node_str = rest[colon_idx + 1:].strip()
            # TODO: Parse node string
            return cls(op=PatchOp.INSERT, target=anchor)
        
        elif op_char == "-":
            # Remove: -id
            return cls(op=PatchOp.REMOVE, target=line[1:])
        
        elif op_char == "!":
            # Replace: !id: node
            rest = line[1:]
            colon_idx = rest.find(":")
            if colon_idx == -1:
                raise ValueError(f"Invalid replace syntax: {line}")
            target = rest[:colon_idx].strip()
            node_str = rest[colon_idx + 1:].strip()
            # TODO: Parse node string
            return cls(op=PatchOp.REPLACE, target=target)
        
        elif op_char == "@":
            # Move: @id → destination
            rest = line[1:]
            arrow_idx = rest.find("→")
            if arrow_idx == -1:
                arrow_idx = rest.find("->")
            if arrow_idx == -1:
                raise ValueError(f"Invalid move syntax: {line}")
            source = rest[:arrow_idx].strip()
            destination = rest[arrow_idx + 2:].strip() if rest[arrow_idx:arrow_idx + 2] == "->" else rest[arrow_idx + 1:].strip()
            return cls(op=PatchOp.MOVE, source=source, destination=destination, target=source)
        
        else:
            raise ValueError(f"Unknown patch operator: {op_char}")
    
    @staticmethod
    def _parse_properties(props_str: str) -> Dict[str, Any]:
        """Parse property string into dictionary."""
        props = {}
        for part in props_str.split():
            if "=" in part:
                key, value = part.split("=", 1)
                props[key] = value
        return props


# =============================================================================
# Component System
# =============================================================================

@dataclass
class SlotDefinition:
    """
    Slot definition for components.
    
    Attributes:
        name: Slot name
        required: Whether the slot is required
    """
    name: str
    required: bool = True


@dataclass
class ComponentDefinition:
    """
    Component definition with inheritance and mixins.
    
    Components are structural templates that can inherit from other
    components and mix in style traits.
    
    Attributes:
        name: Component name
        parent: Parent component name (for inheritance)
        mixins: Style mixin names
        slots: Slot definitions
        default_children: Default children nodes
    """
    name: str
    parent: Optional[str] = None
    mixins: List[str] = field(default_factory=list)
    slots: List[SlotDefinition] = field(default_factory=list)
    default_children: List[UINode] = field(default_factory=list)


# =============================================================================
# UI State Tree
# =============================================================================

@dataclass
class UIStateTree:
    """
    UI State Tree stored in the State Store.
    
    The UI State Tree is the central data structure that the UI Runtime
    renders and maintains. It's addressed by stable IDs for patching.
    
    Attributes:
        root: Root node of the tree
        nodes: Flat map of node IDs to nodes for O(1) lookup
        version: Tree version (incremented on each patch)
        last_updated: Timestamp of last update
    """
    root: UINode
    nodes: Dict[str, UINode] = field(default_factory=dict)
    version: int = 0
    last_updated: float = field(default_factory=time.time)
    
    def __post_init__(self):
        """Build flat node map."""
        self._build_node_map(self.root)
    
    def _build_node_map(self, node: UINode) -> None:
        """Recursively build the node map."""
        if node.id:
            self.nodes[node.id] = node
        for child in node.children:
            self._build_node_map(child)
    
    def find_node(self, node_id: str) -> Optional[UINode]:
        """Find a node by ID."""
        return self.nodes.get(node_id)
    
    def apply_patch(self, patch: PatchOperation) -> bool:
        """
        Apply a patch operation to the tree.
        
        Returns:
            True if patch was applied successfully
        """
        if patch.op == PatchOp.UPDATE:
            return self._apply_update(patch)
        elif patch.op == PatchOp.INSERT:
            return self._apply_insert(patch)
        elif patch.op == PatchOp.REMOVE:
            return self._apply_remove(patch)
        elif patch.op == PatchOp.REPLACE:
            return self._apply_replace(patch)
        elif patch.op == PatchOp.MOVE:
            return self._apply_move(patch)
        return False
    
    def _apply_update(self, patch: PatchOperation) -> bool:
        """Apply update patch."""
        node = self.find_node(patch.target)
        if not node:
            logger.warning(f"Node {patch.target} not found for update")
            return False
        
        if patch.properties:
            node.properties.update(patch.properties)
        
        self.version += 1
        self.last_updated = time.time()
        return True
    
    def _apply_insert(self, patch: PatchOperation) -> bool:
        """Apply insert patch."""
        # Parse anchor (e.g., "footer/append", "footer/0")
        parts = patch.target.split("/")
        if len(parts) < 2:
            logger.warning(f"Invalid insert anchor: {patch.target}")
            return False
        
        parent_id = parts[0]
        anchor = parts[1] if len(parts) > 1 else "append"
        
        parent = self.find_node(parent_id)
        if not parent:
            logger.warning(f"Parent node {parent_id} not found for insert")
            return False
        
        if patch.node:
            if anchor == "append":
                parent.children.append(patch.node)
            elif anchor == "prepend":
                parent.children.insert(0, patch.node)
            elif anchor.isdigit():
                index = int(anchor)
                parent.children.insert(index, patch.node)
            else:
                # Find node with this ID and insert after
                for i, child in enumerate(parent.children):
                    if child.id == anchor:
                        parent.children.insert(i + 1, patch.node)
                        break
            
            patch.node.parent = parent
            if patch.node.id:
                self.nodes[patch.node.id] = patch.node
            
            self.version += 1
            self.last_updated = time.time()
            return True
        
        return False
    
    def _apply_remove(self, patch: PatchOperation) -> bool:
        """Apply remove patch."""
        node = self.find_node(patch.target)
        if not node:
            logger.warning(f"Node {patch.target} not found for removal")
            return False
        
        if node.parent:
            node.parent.children.remove(node)
            if node.id and node.id in self.nodes:
                del self.nodes[node.id]
            
            self.version += 1
            self.last_updated = time.time()
            return True
        
        return False
    
    def _apply_replace(self, patch: PatchOperation) -> bool:
        """Apply replace patch."""
        node = self.find_node(patch.target)
        if not node or not patch.node:
            logger.warning(f"Cannot replace: node or replacement not found")
            return False
        
        if node.parent:
            index = node.parent.children.index(node)
            node.parent.children[index] = patch.node
            patch.node.parent = node.parent
            
            # Update node map
            if node.id and node.id in self.nodes:
                del self.nodes[node.id]
            if patch.node.id:
                self.nodes[patch.node.id] = patch.node
            
            self.version += 1
            self.last_updated = time.time()
            return True
        
        return False
    
    def _apply_move(self, patch: PatchOperation) -> bool:
        """Apply move patch."""
        node = self.find_node(patch.source)
        if not node or not patch.destination:
            logger.warning(f"Cannot move: node or destination not found")
            return False
        
        # Parse destination
        parts = patch.destination.split("/")
        if len(parts) < 2:
            return False
        
        parent_id = parts[0]
        index = int(parts[1]) if parts[1].isdigit() else 0
        
        new_parent = self.find_node(parent_id)
        if not new_parent:
            return False
        
        # Remove from current parent
        if node.parent:
            node.parent.children.remove(node)
        
        # Insert at new position
        if index >= len(new_parent.children):
            new_parent.children.append(node)
        else:
            new_parent.children.insert(index, node)
        
        node.parent = new_parent
        
        self.version += 1
        self.last_updated = time.time()
        return True
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary representation."""
        return {
            "version": self.version,
            "last_updated": self.last_updated,
            "root": self.root.to_dict(),
        }
