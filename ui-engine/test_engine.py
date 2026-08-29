"""
Tests for the AUIL/ASL UI Engine.

Version: 0.1.0
"""

import sys
import os

# Add ui-engine to path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

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
    Reference,
    ReferenceType,
)
from auil_parser import AUILParser, parse_auil
from asl_parser import ASLParser, parse_asl
from patch_protocol import PatchParser, PatchApplicator, parse_patches
from components import ComponentRegistry
from runtime import UIRuntime
from renderer import MockRenderer, TreeRenderer
from mcp_interface import MCPControlInterface
from server import UIEngine, create_engine


def test_models():
    """Test core data models."""
    print("Testing models...")
    
    # Test UINode
    node = UINode(tag="button", id="ok", properties={"label": "OK"})
    assert node.tag == "button"
    assert node.id == "ok"
    assert node.properties["label"] == "OK"
    print("  ✓ UINode created")
    
    # Test auto-generated ID
    node2 = UINode(tag="text")
    assert node2.id is not None
    print("  ✓ Auto-generated ID")
    
    # Test Reference parsing
    ref = Reference.parse("$lambda:video_player.stream")
    assert ref.type == ReferenceType.LAMBDA
    assert ref.path == "video_player.stream"
    print("  ✓ Reference parsing")
    
    # Test DesignToken
    token = DesignToken(
        name="surface.primary",
        value="adaptive(light:#FFFFFFEE dark:#1E1E1EEE)"
    )
    assert token.name == "surface.primary"
    print("  ✓ DesignToken created")
    
    print("✓ Models tests passed\n")


def test_auil_parser():
    """Test AUIL parser."""
    print("Testing AUIL parser...")
    
    parser = AUILParser()
    
    # Test simple tree
    tree = parser.parse('''
stack#root dir=v gap=m
  text(role=title) "Hello World"
  button#ok label=OK on:press=mcp:app.confirm
''')
    
    assert tree.tag == "stack"
    assert tree.id == "root"
    assert len(tree.children) == 2
    
    text_node = tree.children[0]
    assert text_node.tag == "text"
    assert text_node.text_content == "Hello World"
    assert text_node.properties["role"] == "title"
    
    button_node = tree.children[1]
    assert button_node.tag == "button"
    assert button_node.id == "ok"
    assert button_node.properties["label"] == "OK"
    # on:press is parsed as a Reference
    ref = button_node.properties["on:press"]
    assert ref.type == ReferenceType.MCP
    assert ref.path == "app.confirm"
    
    print("  ✓ Simple tree parsed")
    
    # Test mixins
    tree2 = parser.parse('''
stack.Surface.Hoverable#card
  text "Card content"
''')
    assert "Surface" in tree2.mixins
    assert "Hoverable" in tree2.mixins
    print("  ✓ Mixins parsed")
    
    # Test nested structure
    tree3 = parser.parse('''
stack dir=v
  stack dir=h
    text "Left"
    text "Right"
  text "Bottom"
''')
    assert len(tree3.children) == 2
    assert len(tree3.children[0].children) == 2
    print("  ✓ Nested structure parsed")
    
    print("✓ AUIL parser tests passed\n")


def test_asl_parser():
    """Test ASL parser."""
    print("Testing ASL parser...")
    
    parser = ASLParser()
    
    result = parser.parse('''
token surface.primary = adaptive(light:#FFFFFFEE dark:#1E1E1EEE)
token accent = #007AFF

scale radius: sm=6 md=10 lg=16
scale space: xs=4 sm=8 md=12 lg=16 xl=24

motion snappy = spring(stiffness=300 damping=26)
motion gentle = duration(300ms ease=ease-out)

style Surface
  bg=token:surface.primary
  radius=r-lg

style Hoverable
  on:hover => scale=1.02 motion=snappy
  on:press => scale=0.98 motion=snappy
''')
    
    assert "surface.primary" in result["tokens"]
    assert "accent" in result["tokens"]
    print("  ✓ Tokens parsed")
    
    assert "radius" in result["scales"]
    assert "space" in result["scales"]
    print("  ✓ Scales parsed")
    
    assert "snappy" in result["motions"]
    assert "gentle" in result["motions"]
    print("  ✓ Motions parsed")
    
    assert "Surface" in result["styles"]
    assert "Hoverable" in result["styles"]
    print("  ✓ Styles parsed")
    
    hoverable = result["styles"]["Hoverable"]
    assert "hover" in hoverable.transitions
    assert "press" in hoverable.transitions
    print("  ✓ State transitions parsed")
    
    print("✓ ASL parser tests passed\n")


def test_patch_protocol():
    """Test patch protocol."""
    print("Testing patch protocol...")
    
    parser = PatchParser()
    
    # Test update
    op1 = parser.parse("~footer(color=accent style=compact)")
    assert op1 is not None
    assert op1.op == PatchOp.UPDATE
    assert op1.target == "footer"
    assert op1.properties["color"] == "accent"
    print("  ✓ Update patch parsed")
    
    # Test remove
    op2 = parser.parse("-old-banner")
    assert op2 is not None
    assert op2.op == PatchOp.REMOVE
    assert op2.target == "old-banner"
    print("  ✓ Remove patch parsed")
    
    # Test batch parsing
    ops = parser.parse_batch('''
~footer(color=accent)
-old-banner
+footer/append: text(role=caption) "Copyright"
''')
    assert len(ops) == 3
    print("  ✓ Batch parsing works")
    
    # Test tree application
    root = UINode(tag="stack", id="root")
    child = UINode(tag="text", id="footer")
    child.parent = root  # Set parent explicitly
    root.children.append(child)
    tree = UIStateTree(root=root)
    
    # Apply update
    patch = PatchOperation(
        op=PatchOp.UPDATE,
        target="footer",
        properties={"color": "accent"},
    )
    success = tree.apply_patch(patch)
    assert success
    assert child.properties["color"] == "accent"
    print("  ✓ Patch application works")
    
    # Apply remove
    patch2 = PatchOperation(op=PatchOp.REMOVE, target="footer")
    success2 = tree.apply_patch(patch2)
    assert success2
    assert len(root.children) == 0
    print("  ✓ Node removal works")
    
    print("✓ Patch protocol tests passed\n")


def test_components():
    """Test component registry."""
    print("Testing components...")
    
    registry = ComponentRegistry()
    
    # Test default components
    components = registry.list_components()
    assert len(components) > 0
    print(f"  ✓ {len(components)} default components loaded")
    
    # Test getting component
    card = registry.get("Card")
    assert card is not None
    assert card.parent == "Surface"
    print("  ✓ Card component has parent Surface")
    
    # Test mixin resolution
    mixins = registry.resolve_mixins("Card")
    assert "Surface" in mixins
    assert "Hoverable" in mixins
    print("  ✓ Mixins resolved from parent")
    
    # Test custom component
    registry.register(
        "VideoCard",
        parent="Card",
        mixins=["VideoPlayer"],
    )
    video_card = registry.get("VideoCard")
    assert video_card is not None
    assert "VideoPlayer" in video_card.mixins
    assert "Surface" in video_card.mixins  # Inherited
    print("  ✓ Custom component with inheritance")
    
    # Test slot validation
    missing = registry.validate_slots("Field", {"input"})
    assert "label" in missing
    print("  ✓ Slot validation works")
    
    print("✓ Component tests passed\n")


def test_runtime():
    """Test UI runtime."""
    print("Testing runtime...")
    
    runtime = UIRuntime()
    
    # Load AUIL
    root = runtime.load_auil('''
stack#root dir=v gap=m
  text(role=title) "Hello World"
  button#ok label=OK on:press=mcp:app.confirm
''')
    
    assert root.tag == "stack"
    assert len(runtime.get_tree().nodes) > 0
    print("  ✓ AUIL loaded")
    
    # Load ASL
    runtime.load_asl('''
style Surface
  bg=#FFFFFF
  radius=8
''')
    assert "Surface" in runtime._styles
    print("  ✓ ASL loaded")
    
    # Apply patch
    success = runtime.apply_patch("~ok(color=accent)")
    assert success
    node = runtime.find_node("ok")
    assert node.properties.get("color") == "accent"
    print("  ✓ Patch applied")
    
    # Test state transitions
    runtime.set_state("ok", "hover")
    states = runtime.get_active_states("ok")
    assert "hover" in states
    print("  ✓ State transitions work")
    
    # Test stats
    stats = runtime.get_stats()
    assert stats["nodes"] > 0
    print("  ✓ Stats available")
    
    print("✓ Runtime tests passed\n")


def test_renderer():
    """Test renderer."""
    print("Testing renderer...")
    
    renderer = MockRenderer()
    tree_renderer = TreeRenderer(renderer)
    
    # Create a tree
    root = UINode(tag="stack", id="root")
    child = UINode(tag="text", id="text1", properties={"text": "Hello"})
    root.children.append(child)
    tree = UIStateTree(root=root)
    
    # Render
    success = tree_renderer.render(tree)
    assert success
    
    # Check surfaces created
    surfaces = renderer.get_surfaces()
    assert "root" in surfaces
    assert "text1" in surfaces
    print("  ✓ Tree rendered to surfaces")
    
    # Check commands
    commands = renderer.get_commands()
    assert len(commands) > 0
    print(f"  ✓ {len(commands)} render commands generated")
    
    print("✓ Renderer tests passed\n")


def test_mcp_interface():
    """Test MCP interface."""
    print("Testing MCP interface...")
    
    mcp = MCPControlInterface()
    
    # Get available tools
    tools = mcp.get_available_tools()
    assert len(tools) > 0
    print(f"  ✓ {len(tools)} MCP tools available")
    
    # Test render tool
    result = mcp.handle_tool_call("ui.render", {
        "tree": '''
stack#root dir=v
  text(role=title) "Hello World"
  button#ok label=OK
'''
    })
    assert result["success"]
    print("  ✓ ui.render works")
    
    # Test patch tool
    result2 = mcp.handle_tool_call("ui.patch", {
        "patch": "~ok(color=accent)"
    })
    assert result2["success"]
    print("  ✓ ui.patch works")
    
    # Test state tool
    result3 = mcp.handle_tool_call("ui.state", {})
    assert "version" in result3
    print("  ✓ ui.state works")
    
    # Test event tool
    result4 = mcp.handle_tool_call("ui.event", {
        "event_type": "hover",
        "node_id": "ok",
    })
    assert result4["success"]
    assert result4["handled_locally"]
    print("  ✓ ui.event works (real-time handled locally)")
    
    # Test stats tool
    result5 = mcp.handle_tool_call("ui.get_stats", {})
    assert result5["success"]
    print("  ✓ ui.get_stats works")
    
    print("✓ MCP interface tests passed\n")


def test_engine():
    """Test UI engine end-to-end."""
    print("Testing UI engine...")
    
    engine = create_engine()
    
    # Render initial tree
    result = engine.render('''
stack#root dir=v gap=m
  text(role=title) "Video Player"
  media#video type=video src=$lambda:video_player.stream
  stack#controls dir=h gap=s
    button#play label=Play on:press=mcp:video_player.play
    slider#progress max=100 value=@player.position
''')
    assert result["success"]
    print("  ✓ Initial render complete")
    
    # Apply patch
    result2 = engine.patch("~play(color=accent)")
    assert result2["success"]
    print("  ✓ Patch applied")
    
    # Get stats
    stats = engine.get_stats()
    assert stats["nodes"] > 0
    print(f"  ✓ {stats['nodes']} nodes in tree")
    
    # MCP tools
    tools = engine.get_tools()
    assert len(tools) > 0
    print(f"  ✓ {len(tools)} MCP tools available")
    
    print("✓ UI engine tests passed\n")


def test_end_to_end():
    """End-to-end test matching the spec example."""
    print("Testing end-to-end workflow...")
    
    engine = create_engine()
    
    # Step 1: Render initial UI
    print("1. Rendering initial UI...")
    result = engine.render('''
stack#root dir=v
  stack.Surface#player dir=v gap=s
    media#video type=video
    stack#controls dir=h gap=m align=center
      button#play label=Play
      button#pause label=Pause
      slider#progress
  stack.Surface#details dir=v gap=s
    text(role=title) "Video Title"
    text(role=body) "Description text"
''')
    assert result["success"]
    print(f"   ✓ Rendered with {result['stats']['nodes']} nodes")
    
    # Step 2: Apply style patch
    print("2. Applying style patch...")
    result2 = engine.patch('''
~player(bg=token:surface.primary radius=r-lg)
~play(on:press=mcp:video_player.play)
~pause(on:press=mcp:video_player.pause)
''')
    assert result2["success"]
    print("   ✓ Styles applied")
    
    # Step 3: Handle hover event (real-time, local)
    print("3. Handling hover event (real-time)...")
    result3 = engine.handle_mcp_tool("ui.event", {
        "event_type": "hover",
        "node_id": "play",
    })
    assert result3["success"]
    assert result3["handled_locally"]
    print("   ✓ Hover handled locally (no MCP round-trip)")
    
    # Step 4: Handle click event (routes to agent)
    print("4. Handling click event (routes to agent)...")
    result4 = engine.handle_mcp_tool("ui.event", {
        "event_type": "press",
        "node_id": "play",
    })
    assert result4["success"]
    print("   ✓ Click event processed")
    
    # Step 5: Get stats
    print("5. Getting stats...")
    stats = engine.get_stats()
    print(f"   ✓ {stats['nodes']} nodes, {stats['styles']} styles, {stats['patches_applied']} patches")
    
    print("\n✓ End-to-end test passed\n")


if __name__ == "__main__":
    print("=" * 60)
    print("AUIL/ASL UI Engine - Test Suite")
    print("=" * 60 + "\n")
    
    test_models()
    test_auil_parser()
    test_asl_parser()
    test_patch_protocol()
    test_components()
    test_runtime()
    test_renderer()
    test_mcp_interface()
    test_engine()
    test_end_to_end()
    
    print("=" * 60)
    print("All tests passed!")
    print("=" * 60)
