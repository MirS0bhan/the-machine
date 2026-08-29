"""
AUIL/ASL UI Engine

A declarative UI language and runtime for the Agent-Native OS.

Architecture:
    - AUIL (Agent UI Layout): Structure language for UI trees
    - ASL (Agent Style Language): Style and motion language
    - Patch Protocol: Agent-to-UI update mechanism
    - UI Runtime: Tree management and rendering coordination
    - MCP Interface: Agent interaction via Model Context Protocol

Version: 0.1.0
"""

from server import UIEngine, create_engine
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
    PrimitiveTag,
    TextRole,
    MediaType,
    ChartType,
    Reference,
    ReferenceType,
)
from auil_parser import AUILParser, parse_auil
from asl_parser import ASLParser, parse_asl
from patch_protocol import PatchParser, PatchApplicator, parse_patches
from runtime import UIRuntime
from components import ComponentRegistry
from renderer import AbstractRenderer, MockRenderer, TreeRenderer, RenderCommand
from mcp_interface import MCPControlInterface

__version__ = "0.1.0"
__author__ = "UI Engine Team"

__all__ = [
    # Main server
    "UIEngine",
    "create_engine",
    
    # Models
    "UINode",
    "UIStateTree",
    "PatchOperation",
    "PatchOp",
    "StyleMixin",
    "StateTransition",
    "DesignToken",
    "MotionCurve",
    "EventType",
    "PrimitiveTag",
    "TextRole",
    "MediaType",
    "ChartType",
    "Reference",
    "ReferenceType",
    
    # Parsers
    "AUILParser",
    "parse_auil",
    "ASLParser",
    "parse_asl",
    "PatchParser",
    "PatchApplicator",
    "parse_patches",
    
    # Runtime
    "UIRuntime",
    
    # Components
    "ComponentRegistry",
    
    # Renderer
    "AbstractRenderer",
    "MockRenderer",
    "TreeRenderer",
    "RenderCommand",
    
    # MCP
    "MCPControlInterface",
]
