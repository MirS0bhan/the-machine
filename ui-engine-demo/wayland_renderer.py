"""
Wayland Renderer Implementation for the UI Engine.

Implements AbstractRenderer as a terminal-based renderer for development
and testing. In production, this would be replaced by a Wayland compositor
renderer (wlroots-based, separate project).

The renderer translates RenderCommands into terminal output, handling
text, buttons, fields, and layout containers. It captures keyboard input
and routes events back to the UIRuntime.

Architecture Reference:
    - §7.2 of docs/spec.md (Renderer — external project)
    - §1 of docs/spec.md (Real-time constraint)

Version: 0.1.0
"""

import sys
import logging
from typing import Any, Callable, Dict, List, Optional
from dataclasses import dataclass, field

logger = logging.getLogger(__name__)


@dataclass
class TerminalSurface:
    """A rendered surface in the terminal."""
    surface_id: str
    tag: str
    properties: Dict[str, Any] = field(default_factory=dict)
    x: int = 0
    y: int = 0
    width: int = 40
    height: int = 1
    focused: bool = False


class WaylandRenderer:
    """
    Terminal-based renderer implementing AbstractRenderer interface.

    Translates RenderCommands into ANSI terminal output. This is the
    development/testing renderer — production uses wlroots compositor.

    Implements create_surface, update_surface, destroy_surface,
    commit_batch, flush, and get_surface_state.
    """

    def __init__(self):
        self._surfaces: Dict[str, TerminalSurface] = {}
        self._event_handler: Optional[Callable[[str, Dict[str, Any]], None]] = None
        self._dirty = False
        self._cursor_y = 0

    def set_event_handler(self, handler: Callable[[str, Dict[str, Any]], None]) -> None:
        self._event_handler = handler

    def create_surface(self, surface_id: str, properties: Dict[str, Any]) -> bool:
        tag = properties.get("tag", "text")
        text = properties.get("text", "")
        placeholder = properties.get("placeholder", "")
        label = properties.get("label", "")
        direction = properties.get("dir", properties.get("orientation", "v"))

        width = max(len(text), len(placeholder), len(label), 20) + 4

        surface = TerminalSurface(
            surface_id=surface_id,
            tag=tag,
            properties=properties,
            x=2,
            y=self._cursor_y,
            width=width,
            height=1 if tag not in ("stack",) else 1,
        )

        self._surfaces[surface_id] = surface
        self._dirty = True
        return True

    def update_surface(self, surface_id: str, properties: Dict[str, Any]) -> bool:
        if surface_id not in self._surfaces:
            return False
        surface = self._surfaces[surface_id]
        surface.properties.update(properties)
        self._dirty = True
        return True

    def destroy_surface(self, surface_id: str) -> bool:
        if surface_id in self._surfaces:
            del self._surfaces[surface_id]
            self._dirty = True
            return True
        return False

    def commit_batch(self, commands: list) -> bool:
        for cmd in commands:
            if cmd.type == "create":
                self.create_surface(cmd.node_id, cmd.properties)
            elif cmd.type == "update":
                self.update_surface(cmd.node_id, cmd.properties)
            elif cmd.type == "destroy":
                self.destroy_surface(cmd.node_id)
        return True

    def flush(self) -> bool:
        if not self._dirty:
            return True
        self._render_frame()
        self._dirty = False
        return True

    def get_surface_state(self, surface_id: str) -> Optional[Dict[str, Any]]:
        surface = self._surfaces.get(surface_id)
        if surface:
            return {"surface_id": surface.surface_id, "tag": surface.tag, "properties": surface.properties}
        return None

    def _render_frame(self) -> None:
        sys.stdout.write("\033[2J\033[H")
        lines: List[str] = []
        render_order = sorted(self._surfaces.values(), key=lambda s: s.y)

        for surface in render_order:
            tag = surface.tag
            props = surface.properties
            text = props.get("text", "")
            label = props.get("label", "")
            placeholder = props.get("placeholder", "")
            focused = surface.focused

            if tag == "stack":
                dir_char = props.get("dir", props.get("orientation", "v"))
                lines.append(f"  [{surface.surface_id}] stack({'v' if dir_char in ('v', 'vertical') else 'h'})")
            elif tag == "text":
                role = props.get("role", "")
                display = text or label
                if role == "title":
                    lines.append(f"\033[1;36m  {display}\033[0m")
                elif role == "label":
                    lines.append(f"  {display}")
                elif role == "caption":
                    lines.append(f"\033[90m  {display}\033[0m")
                else:
                    lines.append(f"  {display}")
            elif tag == "button":
                cursor = ">" if focused else " "
                lines.append(f"  {cursor} [{label or text}]")
            elif tag == "field" or tag == "entry":
                cursor = ">" if focused else " "
                value = text or placeholder
                lines.append(f"  {cursor} {value}")
            elif tag == "label":
                lines.append(f"  {text}")

        print("\n".join(lines))

    def get_surfaces(self) -> Dict[str, Dict[str, Any]]:
        return {
            sid: {"tag": s.tag, "properties": s.properties}
            for sid, s in self._surfaces.items()
        }

    def clear(self) -> None:
        self._surfaces.clear()
        self._cursor_y = 0

    def handle_key(self, key: str) -> Optional[Dict[str, Any]]:
        if self._event_handler:
            event = {"key": key}
            self._event_handler("key", event)
            return event
        return None
