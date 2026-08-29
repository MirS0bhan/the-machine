"""
Patch Protocol Implementation.

This module implements the patch protocol for UI updates. The agent's
steady-state output is never a full tree — it's one or more patch ops
against existing IDs.

Architecture Reference:
    - §2.2 of docs/spec.md (Full tree is last resort)
    - §5 of docs/spec.md (Patch protocol)

Version: 0.1.0
"""

import logging
import re
from typing import Any, Dict, List, Optional, Tuple
from models import (
    UINode,
    PatchOperation,
    PatchOp,
    Property,
    UIStateTree,
)

logger = logging.getLogger(__name__)


# Patch operation patterns
PATCH_PATTERNS = {
    PatchOp.UPDATE: re.compile(r'^~(\w[\w/-]*)\(([^)]*)\)$'),
    PatchOp.INSERT: re.compile(r'^\+([\w/-]+)(?::\s*(.*))?$'),
    PatchOp.REMOVE: re.compile(r'^-(\w[\w/-]*)$'),
    PatchOp.REPLACE: re.compile(r'^!(\w[\w/-]*):\s*(.+)$'),
    PatchOp.MOVE: re.compile(r'^@(\w[\w/-]*)\s*→\s*(.+)$'),
}


class PatchParser:
    """
    Parser for patch operations.
    
    Patch ops use single-character prefixes:
    - ~ Update props in place
    - + Insert node
    - - Remove node
    - ! Replace subtree
    - @ Move/reorder node
    
    Example:
        >>> parser = PatchParser()
        >>> ops = parser.parse_batch('''
        ... ~footer(color=accent)style=compact
        ... +footer/append: text(role=caption) "Copyright"
        ... -old-banner
        ... ''')
    """
    
    def __init__(self):
        """Initialize the parser."""
        self._errors: List[str] = []
        self._line_number = 0
    
    def parse(self, line: str) -> Optional[PatchOperation]:
        """
        Parse a single patch operation line.
        
        Args:
            line: Patch operation line
            
        Returns:
            PatchOperation if parsed successfully, None otherwise
        """
        line = line.strip()
        if not line or line.startswith('//'):
            return None
        
        self._line_number += 1
        
        # Determine operation type
        if line.startswith('~'):
            return self._parse_update(line)
        elif line.startswith('+'):
            return self._parse_insert(line)
        elif line.startswith('-'):
            return self._parse_remove(line)
        elif line.startswith('!'):
            return self._parse_replace(line)
        elif line.startswith('@'):
            return self._parse_move(line)
        else:
            self._errors.append(f"Line {self._line_number}: Unknown patch operator: {line[0]}")
            return None
    
    def parse_batch(self, source: str) -> List[PatchOperation]:
        """
        Parse multiple patch operations.
        
        Args:
            source: Source code with multiple patch ops
            
        Returns:
            List of parsed PatchOperations
        """
        self._errors = []
        self._line_number = 0
        
        ops = []
        for line in source.split('\n'):
            op = self.parse(line)
            if op:
                ops.append(op)
        
        if self._errors:
            logger.warning(f"Patch parse completed with {len(self._errors)} warnings")
        
        return ops
    
    def _parse_update(self, line: str) -> Optional[PatchOperation]:
        """
        Parse update patch: ~id(prop1=val1 prop2=val2)
        
        Args:
            line: Update line
            
        Returns:
            PatchOperation or None
        """
        match = PATCH_PATTERNS[PatchOp.UPDATE].match(line)
        if not match:
            self._errors.append(f"Line {self._line_number}: Invalid update syntax: {line}")
            return None
        
        target = match.group(1)
        props_str = match.group(2)
        
        properties = {}
        if props_str:
            for key, value in self._tokenize_properties(props_str):
                properties[key] = self._parse_value(value)
        
        return PatchOperation(
            op=PatchOp.UPDATE,
            target=target,
            properties=properties,
        )
    
    def _parse_insert(self, line: str) -> Optional[PatchOperation]:
        """
        Parse insert patch: +anchor: node or +anchor/position
        
        Args:
            line: Insert line
            
        Returns:
            PatchOperation or None
        """
        match = PATCH_PATTERNS[PatchOp.INSERT].match(line)
        if not match:
            # Also accept +anchor/path node-def without colon separator.
            alt = re.match(r'^\+([\w/-]+)\s+(.+)$', line)
            if alt:
                target = alt.group(1)
                node_str = alt.group(2)
            else:
                self._errors.append(f"Line {self._line_number}: Invalid insert syntax: {line}")
                return None
        else:
            target = match.group(1)
            node_str = match.group(2)
        
        node = None
        if node_str:
            # Parse node string (simplified)
            node = self._parse_node_string(node_str)
        
        return PatchOperation(
            op=PatchOp.INSERT,
            target=target,
            node=node,
        )
    
    def _parse_remove(self, line: str) -> Optional[PatchOperation]:
        """
        Parse remove patch: -id
        
        Args:
            line: Remove line
            
        Returns:
            PatchOperation or None
        """
        match = PATCH_PATTERNS[PatchOp.REMOVE].match(line)
        if not match:
            self._errors.append(f"Line {self._line_number}: Invalid remove syntax: {line}")
            return None
        
        target = match.group(1)
        
        return PatchOperation(
            op=PatchOp.REMOVE,
            target=target,
        )
    
    def _parse_replace(self, line: str) -> Optional[PatchOperation]:
        """
        Parse replace patch: !id: new_node
        
        Args:
            line: Replace line
            
        Returns:
            PatchOperation or None
        """
        match = PATCH_PATTERNS[PatchOp.REPLACE].match(line)
        if not match:
            self._errors.append(f"Line {self._line_number}: Invalid replace syntax: {line}")
            return None
        
        target = match.group(1)
        node_str = match.group(2)
        
        node = None
        if node_str:
            node = self._parse_node_string(node_str)
        
        return PatchOperation(
            op=PatchOp.REPLACE,
            target=target,
            node=node,
        )
    
    def _parse_move(self, line: str) -> Optional[PatchOperation]:
        """
        Parse move patch: @id → destination
        
        Args:
            line: Move line
            
        Returns:
            PatchOperation or None
        """
        match = PATCH_PATTERNS[PatchOp.MOVE].match(line)
        if not match:
            self._errors.append(f"Line {self._line_number}: Invalid move syntax: {line}")
            return None
        
        source = match.group(1)
        destination = match.group(2).strip()
        
        return PatchOperation(
            op=PatchOp.MOVE,
            source=source,
            destination=destination,
            target=source,
        )
    
    def _parse_node_string(self, node_str: str) -> Optional[UINode]:
        """
        Parse a node string into a UINode.
        
        This is a simplified parser for inline node definitions.
        
        Args:
            node_str: Node string
            
        Returns:
            UINode or None
        """
        # Simple parsing: tag#id mixins props "text"
        # For now, just create a basic node
        parts = node_str.split()
        if not parts:
            return None
        
        tag = parts[0]
        node_id = None
        mixins = []
        properties = {}
        text_content = None
        
        for part in parts[1:]:
            if part.startswith('#'):
                node_id = part[1:]
            elif part.startswith('.'):
                mixins.append(part[1:])
            elif '=' in part:
                key, value = part.split('=', 1)
                properties[key] = self._parse_value(value)
            elif part.startswith('"') and part.endswith('"'):
                text_content = part[1:-1]
        
        return UINode(
            tag=tag,
            id=node_id,
            mixins=mixins,
            properties=properties,
            text_content=text_content,
        )
    
    def _tokenize_properties(self, props_str: str) -> List[Tuple[str, str]]:
        """Tokenize key=value pairs, respecting quoted values with spaces."""
        tokens: List[str] = []
        current: List[str] = []
        in_quotes = False
        for char in props_str:
            if char == '"':
                in_quotes = not in_quotes
                current.append(char)
            elif char == ' ' and not in_quotes:
                if current:
                    tokens.append(''.join(current))
                    current = []
            else:
                current.append(char)
        if current:
            tokens.append(''.join(current))

        pairs: List[Tuple[str, str]] = []
        for token in tokens:
            if '=' in token:
                key, value = token.split('=', 1)
                pairs.append((key, value))
        return pairs

    def _parse_value(self, value: str) -> Any:
        """
        Parse a value string.
        
        Args:
            value: Value string
            
        Returns:
            Parsed value
        """
        if value.startswith('"') and value.endswith('"'):
            value = value[1:-1]

        # Check for references
        if value.startswith(("$lambda:", "mcp:", "@")):
            return {"type": "reference", "value": value}
        
        # Try to parse as number
        try:
            if '.' in value:
                return float(value)
            else:
                return int(value)
        except ValueError:
            pass
        
        # Try to parse as bool
        if value.lower() in ("true", "yes"):
            return True
        elif value.lower() in ("false", "no"):
            return False
        
        # String value
        return value
    
    def get_errors(self) -> List[str]:
        """Get parse errors."""
        return self._errors.copy()


class PatchApplicator:
    """
    Applies patch operations to a UI State Tree.
    
    The applicator handles the mechanics of applying patches,
    including validation, ordering, and error recovery.
    
    Example:
        >>> applicator = PatchApplicator()
        >>> tree = UIStateTree(root=root_node)
        >>> ops = parse_patches("~footer(color=accent)")
        >>> success = applicator.apply(tree, ops)
    """
    
    def __init__(self):
        """Initialize the applicator."""
        self._applied_count = 0
        self._failed_count = 0
    
    def apply(self, tree: UIStateTree, ops: List[PatchOperation]) -> bool:
        """
        Apply a list of patch operations to a tree.
        
        Args:
            tree: UI State Tree to patch
            ops: List of patch operations
            
        Returns:
            True if all patches applied successfully
        """
        all_success = True
        
        for op in ops:
            success = tree.apply_patch(op)
            if success:
                self._applied_count += 1
                logger.debug(f"Applied patch: {op.op.value}{op.target}")
            else:
                self._failed_count += 1
                all_success = False
                logger.warning(f"Failed to apply patch: {op.op.value}{op.target}")
        
        return all_success
    
    def get_stats(self) -> Dict[str, int]:
        """Get applicator statistics."""
        return {
            "applied": self._applied_count,
            "failed": self._failed_count,
        }
    
    def reset_stats(self) -> None:
        """Reset statistics."""
        self._applied_count = 0
        self._failed_count = 0


def parse_patches(source: str) -> List[PatchOperation]:
    """
    Convenience function to parse patch operations.
    
    Args:
        source: Patch operations source code
        
    Returns:
        List of PatchOperations
        
    Example:
        >>> from patch_protocol import parse_patches
        >>> ops = parse_patches("~footer(color=accent)")
    """
    parser = PatchParser()
    return parser.parse_batch(source)
