"""
UI Engine Demo — Interactive App.

Wires together:
  - UIRuntime (AUIL parsing, patch protocol, state management)
  - WaylandRenderer (terminal-based renderer implementing AbstractRenderer)
  - MCP handlers (button clicks, text input)

The app presents a text field, a submit button, and an output label.
Typing in the field and pressing submit copies the input text to the output.

Usage:
  cd ui-engine-demo && poetry install
  poetry run python demo.py
"""

import sys
import os
import logging

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "ui-engine"))

from auil_parser import parse_auil
from runtime import UIRuntime
from patch_protocol import parse_patches
from wayland_renderer import WaylandRenderer

logging.basicConfig(level=logging.WARNING)
logger = logging.getLogger(__name__)


class App:
    def __init__(self):
        self.runtime = UIRuntime()
        self.renderer = WaylandRenderer()

        self.input_text = ""
        self.output_text = "Output will appear here"
        self.running = True

        self.renderer.set_event_handler(self._on_event)

    def start(self) -> None:
        self._load_ui()
        self._render()
        self._print_help()

    def _load_ui(self) -> None:
        auil_source = open(os.path.join(os.path.dirname(__file__), "demo.auil")).read()
        root = self.runtime.load_auil(auil_source)

    def _render(self) -> None:
        tree = self.runtime.get_tree()
        if not tree:
            return

        self.renderer.clear()

        for node_id, node in tree.nodes.items():
            props = dict(node.properties)
            props["tag"] = node.tag
            if node.text_content:
                props["text"] = node.text_content
            self.renderer.create_surface(node_id, props)

        self.renderer.flush()

    def _apply_patch(self, patch_source: str) -> bool:
        success = self.runtime.apply_patch(patch_source)
        if success:
            self._render()
        return success

    def _on_event(self, event_type: str, event_data: dict) -> None:
        if event_type == "key":
            key = event_data.get("key", "")

            if key == "q":
                self.running = False
                print("\nGoodbye!")
                return

            if key == "\n":
                self._on_submit()
                return

            if key == "tab":
                self._on_tab()
                return

            if key == "backspace":
                self.input_text = self.input_text[:-1]
            else:
                if len(key) == 1:
                    self.input_text += key

            self._apply_patch(f"~input-entry(text=\"{self.input_text}\")")

    def _on_submit(self) -> None:
        if self.input_text.strip():
            self.output_text = f"You typed: {self.input_text}"
            self._apply_patch(f"~output-label(text=\"{self.output_text}\")")
            print(f"\n  [Submit] Output: {self.output_text}\n")
        else:
            self.output_text = "Nothing to submit!"
            self._apply_patch(f"~output-label(text=\"{self.output_text}\")")

    def _on_tab(self) -> None:
        self._apply_patch(f"~submit-btn(focused=true)")

    def _print_help(self) -> None:
        print("\n  Type to input text | Enter = submit | Tab = focus | q = quit\n")

    def run(self) -> None:
        self.start()

        while self.running:
            try:
                import tty
                import termios
                fd = sys.stdin.fileno()
                old_settings = termios.tcgetattr(fd)
                try:
                    tty.setraw(fd)
                    ch = sys.stdin.read(1)

                    if ch == "\x1b":
                        seq = sys.stdin.read(2)
                        if seq == "[A":
                            self._on_event("key", {"key": "up"})
                        elif seq == "[B":
                            self._on_event("key", {"key": "down"})
                        elif seq == "[C":
                            self._on_event("key", {"key": "right"})
                        elif seq == "[D":
                            self._on_event("key", {"key": "left"})
                        continue

                    if ch == "\x03":
                        self.running = False
                        print("\nGoodbye!")
                        break

                    if ch == "\x7f":
                        self._on_event("key", {"key": "backspace"})
                    elif ch == "\r" or ch == "\n":
                        self._on_event("key", {"key": "\n"})
                    elif ch == "\t":
                        self._on_event("key", {"key": "tab"})
                    elif ch.isalpha() or ch.isdigit() or ch in " .,!?-_@#$%&*()[]{}|\\/:;\"'":
                        self._on_event("key", {"key": ch})
                    else:
                        self._on_event("key", {"key": ch})
                finally:
                    termios.tcsetattr(fd, termios.TCSADRAIN, old_settings)
            except KeyboardInterrupt:
                self.running = False
                print("\nGoodbye!")
                break


def main():
    print("=" * 50)
    print("  UI Engine Demo (Pure Wayland Renderer)")
    print("=" * 50)

    app = App()
    app.run()


if __name__ == "__main__":
    main()
