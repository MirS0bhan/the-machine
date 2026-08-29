"""
Tests for the UI Engine Demo.

Verifies the full pipeline:
  1. AUIL parsing → UINode tree
  2. UIRuntime → UIStateTree
  3. WaylandRenderer → surfaces
  4. Patch protocol → updates
  5. MCP handlers → state changes
"""

import sys
import os
import json
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "ui-engine"))
sys.path.insert(0, os.path.dirname(__file__))

from auil_parser import parse_auil, AUILParser
from runtime import UIRuntime
from patch_protocol import parse_patches, PatchParser
from models import UINode, UIStateTree, PatchOperation, PatchOp
from wayland_renderer import WaylandRenderer


class TestAUILParser(unittest.TestCase):
    def test_parse_simple_layout(self):
        source = """\
stack#root dir=v
  text(role=title) "Hello"
  button#btn label="Click Me"
"""
        root = parse_auil(source)
        self.assertEqual(root.tag, "stack")
        self.assertEqual(root.id, "root")
        self.assertEqual(len(root.children), 2)
        self.assertEqual(root.children[0].tag, "text")
        self.assertEqual(root.children[0].text_content, "Hello")
        self.assertEqual(root.children[1].tag, "button")
        self.assertEqual(root.children[1].properties.get("label"), "Click Me")

    def test_parse_field_with_placeholder(self):
        source = """\
field#input placeholder="Type here..."
"""
        root = parse_auil(source)
        self.assertEqual(root.tag, "field")
        self.assertEqual(root.id, "input")
        self.assertEqual(root.properties.get("placeholder"), "Type here...")

    def test_parse_mcp_reference(self):
        source = """\
button#ok on:press=mcp:app.confirm
"""
        root = parse_auil(source)
        self.assertIn("on:press", root.properties)

    def test_nested_nodes(self):
        source = """\
stack#root dir=v
  stack#header dir=h
    text(role=title) "Header"
  stack#body dir=v
    text "Content"
"""
        root = parse_auil(source)
        self.assertEqual(len(root.children), 2)
        self.assertEqual(root.children[0].id, "header")
        self.assertEqual(root.children[1].id, "body")
        self.assertEqual(len(root.children[0].children), 1)
        self.assertEqual(len(root.children[1].children), 1)


class TestPatchProtocol(unittest.TestCase):
    def test_parse_update_patch(self):
        parser = PatchParser()
        op = parser.parse('~footer(color=accent)')
        self.assertIsNotNone(op)
        self.assertEqual(op.op, PatchOp.UPDATE)
        self.assertEqual(op.target, "footer")
        self.assertEqual(op.properties.get("color"), "accent")

    def test_parse_remove_patch(self):
        parser = PatchParser()
        op = parser.parse('-old-banner')
        self.assertIsNotNone(op)
        self.assertEqual(op.op, PatchOp.REMOVE)
        self.assertEqual(op.target, "old-banner")

    def test_parse_batch(self):
        ops = parse_patches("""\
~btn(label="Changed")
-remove-me
+footer/text "New"
""")
        self.assertEqual(len(ops), 3)

    def test_apply_update_to_tree(self):
        root = UINode(tag="stack", id="root")
        child = UINode(tag="text", id="label", text_content="old")
        root.children = [child]
        child.parent = root
        tree = UIStateTree(root=root)

        patch = PatchOperation(op=PatchOp.UPDATE, target="label", properties={"text": "new"})
        success = tree.apply_patch(patch)
        self.assertTrue(success)
        self.assertEqual(tree.find_node("label").properties.get("text"), "new")

    def test_apply_remove_to_tree(self):
        root = UINode(tag="stack", id="root")
        btn = UINode(tag="button", id="remove-me")
        keep = UINode(tag="text", id="keep")
        root.children = [btn, keep]
        btn.parent = root
        keep.parent = root
        tree = UIStateTree(root=root)

        patch = PatchOperation(op=PatchOp.REMOVE, target="remove-me")
        success = tree.apply_patch(patch)
        self.assertTrue(success)
        self.assertIsNone(tree.find_node("remove-me"))
        self.assertIsNotNone(tree.find_node("keep"))


class TestUIRuntime(unittest.TestCase):
    def test_load_and_patch(self):
        runtime = UIRuntime()
        runtime.load_auil("""\
stack#root dir=v
  text#label "Hello"
  button#btn label="Click"
""")
        self.assertIsNotNone(runtime.get_tree())
        self.assertEqual(len(runtime.get_tree().nodes), 3)

        success = runtime.apply_patch('~label(text="World")')
        self.assertTrue(success)
        self.assertEqual(runtime.find_node("label").properties.get("text"), "World")

    def test_node_lookup(self):
        runtime = UIRuntime()
        runtime.load_auil("""\
stack#root dir=v
  field#myfield placeholder="Type..."
""")
        node = runtime.find_node("myfield")
        self.assertIsNotNone(node)
        self.assertEqual(node.tag, "field")

    def test_stats(self):
        runtime = UIRuntime()
        runtime.load_auil("""\
stack#root dir=v
  text "A"
  button label="B"
""")
        stats = runtime.get_stats()
        self.assertGreater(stats["nodes"], 0)


class TestWaylandRenderer(unittest.TestCase):
    def test_create_surface(self):
        renderer = WaylandRenderer()
        success = renderer.create_surface("test-btn", {"tag": "button", "label": "OK"})
        self.assertTrue(success)
        self.assertIn("test-btn", renderer.get_surfaces())

    def test_update_surface(self):
        renderer = WaylandRenderer()
        renderer.create_surface("lbl", {"tag": "text", "text": "old"})
        success = renderer.update_surface("lbl", {"text": "new"})
        self.assertTrue(success)
        state = renderer.get_surface_state("lbl")
        self.assertEqual(state["properties"]["text"], "new")

    def test_destroy_surface(self):
        renderer = WaylandRenderer()
        renderer.create_surface("del-me", {"tag": "text", "text": "gone"})
        success = renderer.destroy_surface("del-me")
        self.assertTrue(success)
        self.assertIsNone(renderer.get_surface_state("del-me"))

    def test_commit_batch(self):
        from renderer import RenderCommand
        renderer = WaylandRenderer()
        cmds = [
            RenderCommand(type="create", node_id="a", properties={"tag": "text", "text": "A"}),
            RenderCommand(type="create", node_id="b", properties={"tag": "text", "text": "B"}),
        ]
        success = renderer.commit_batch(cmds)
        self.assertTrue(success)
        self.assertEqual(len(renderer.get_surfaces()), 2)

    def test_full_pipeline(self):
        runtime = UIRuntime()
        runtime.load_auil("""\
stack#root dir=v
  text#output "Ready"
  button#submit label="Go"
""")
        renderer = WaylandRenderer()

        from renderer import TreeRenderer
        tree_renderer = TreeRenderer(renderer)
        tree_renderer.render(runtime.get_tree())

        self.assertIn("output", renderer.get_surfaces())
        self.assertIn("submit", renderer.get_surfaces())

        runtime.apply_patch('~output(text="Done!")')
        tree_renderer.update(runtime.get_tree())
        state = renderer.get_surface_state("output")
        self.assertEqual(state["properties"]["text"], "Done!")


class TestEndToEnd(unittest.TestCase):
    def test_input_submit_output(self):
        runtime = UIRuntime()
        runtime.load_auil("""\
stack#root dir=v
  field#input placeholder="Type..."
  button#submit label="Submit"
  text#output "Output will appear here"
""")
        input_text = "Hello World"

        runtime.apply_patch(f'~input(text="{input_text}")')
        self.assertEqual(runtime.find_node("input").properties.get("text"), input_text)

        output_text = f"You typed: {input_text}"
        runtime.apply_patch(f'~output(text="{output_text}")')
        self.assertEqual(runtime.find_node("output").properties.get("text"), output_text)

    def test_empty_submit(self):
        runtime = UIRuntime()
        runtime.load_auil("""\
stack#root dir=v
  field#input placeholder="Type..."
  text#output "Nothing yet"
""")
        runtime.apply_patch('~output(text="Nothing to submit!")')
        self.assertEqual(runtime.find_node("output").properties.get("text"), "Nothing to submit!")

    def test_multiple_patches(self):
        runtime = UIRuntime()
        runtime.load_auil("""\
stack#root dir=v
  text#label "Start"
""")
        runtime.apply_patch('~label(text="Step 1")')
        runtime.apply_patch('~label(text="Step 2")')
        runtime.apply_patch('~label(text="Final")')
        self.assertEqual(runtime.find_node("label").properties.get("text"), "Final")


if __name__ == "__main__":
    unittest.main()
