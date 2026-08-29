"""
AUIL (Agent UI Layout) Parser.

This module implements the parser for AUIL, a line-oriented, indentation-based
language for describing UI structure. The parser is single-pass with an
indentation stack, requiring no backtracking.

Architecture Reference:
    - §2 of docs/spec.md (AUIL — structure language)

Version: 0.1.0
"""

import re
import logging
from typing import Any, Dict, List, Optional, Tuple
from models import (
    UINode,
    Property,
    Reference,
    ReferenceType,
    PrimitiveTag,
    TextRole,
    MediaType,
    ChartType,
    LayoutDirection,
)

logger = logging.getLogger(__name__)


# Fixed primitive tags
PRIMITIVE_TAGS = {tag.value for tag in PrimitiveTag}

# Regex patterns
TAG_PATTERN = re.compile(
    r'^([a-zA-Z][a-zA-Z0-9]*)'  # tag
    r'((?:\.[a-zA-Z][a-zA-Z0-9_]*)*)?'  # optional .mixin1.mixin2
    r'(?:#([a-zA-Z0-9_-]+))?'   # optional #id
    r'(?:\(([^)]*)\))?'         # optional (props) - parenthesized
    r'(?:\s+(.+))?$'            # optional remaining content (props or text)
)


class AUILParser:
    """
    Parser for AUIL (Agent UI Layout).
    
    AUIL is a line-oriented, indentation-based language where:
    - Line format: tag[#id][.mixin1.mixin2](prop=val ...)
    - Indentation = nesting (2 spaces per level)
    - No closing brackets needed
    - Single-pass, no backtracking
    
    Example:
        >>> parser = AUILParser()
        >>> tree = parser.parse('''
        ... stack#root dir=v gap=m
        ...   text(role=title) "Hello World"
        ...   button#ok label=OK on:press=mcp:app.confirm
        ... ''')
    """
    
    def __init__(self, indent_size: int = 2):
        """
        Initialize the parser.
        
        Args:
            indent_size: Number of spaces per indentation level
        """
        self.indent_size = indent_size
        self._line_number = 0
        self._errors: List[str] = []
    
    def parse(self, source: str) -> UINode:
        """
        Parse AUIL source into a UINode tree.
        
        Args:
            source: AUIL source code
            
        Returns:
            Root UINode of the parsed tree
            
        Raises:
            SyntaxError: If the source contains syntax errors
        """
        lines = source.split('\n')
        self._line_number = 0
        self._errors = []
        
        # Filter empty lines and track non-empty lines
        non_empty_lines = [(i + 1, line) for i, line in enumerate(lines) if line.strip()]
        
        if not non_empty_lines:
            return UINode(tag="stack", id="empty")
        
        # Parse using indentation stack
        root = self._parse_block(non_empty_lines, 0, len(non_empty_lines), 0)
        
        if self._errors:
            logger.warning(f"Parse completed with {len(self._errors)} warnings")
        
        return root
    
    def _parse_block(
        self,
        lines: List[Tuple[int, str]],
        start: int,
        end: int,
        base_indent: int
    ) -> UINode:
        """
        Parse a block of lines at the same indentation level.
        
        Args:
            lines: List of (line_number, line_content) tuples
            start: Start index in lines
            end: End index in lines
            base_indent: Base indentation level
            
        Returns:
            Root UINode of the block
        """
        if start >= end:
            return UINode(tag="stack", id="empty-block")
        
        # Parse first line as the root of this block
        line_num, line_content = lines[start]
        self._line_number = line_num
        
        node = self._parse_line(line_content)
        
        # Find children (lines with greater indentation)
        children_start = start + 1
        child_indent = None
        
        for i in range(children_start, end):
            line_num, line_content = lines[i]
            indent = self._get_indent(line_content)
            
            if child_indent is None:
                if indent > base_indent:
                    child_indent = indent
                else:
                    break
            
            if indent == child_indent:
                # Find the extent of this child block
                block_end = i + 1
                while block_end < end:
                    next_indent = self._get_indent(lines[block_end][1])
                    if next_indent <= child_indent:
                        break
                    block_end += 1
                
                # Parse child block
                child = self._parse_block(lines, i, block_end, child_indent)
                child.parent = node
                node.children.append(child)
                
                # Skip to end of child block
                i = block_end - 1
        
        return node
    
    def _parse_line(self, line: str) -> UINode:
        """
        Parse a single AUIL line into a UINode.
        
        Format: tag[#id][.mixin1.mixin2](prop=val ...) ["text content"]
        
        Args:
            line: Line content (without leading whitespace)
            
        Returns:
            Parsed UINode
        """
        line = line.strip()
        if not line:
            return UINode(tag="stack", id="empty-line")
        
        # Try to match the tag pattern
        match = TAG_PATTERN.match(line)
        if not match:
            self._errors.append(f"Line {self._line_number}: Invalid syntax: {line}")
            return UINode(tag="stack", id=f"error-{self._line_number}")
        
        tag = match.group(1)
        mixins_str = match.group(2)
        node_id = match.group(3)
        paren_props_str = match.group(4)
        remaining = match.group(5)
        
        # Parse mixins
        mixins = []
        if mixins_str:
            mixins = [m for m in mixins_str.split('.') if m]
        
        # Parse properties from parentheses
        properties = {}
        if paren_props_str:
            properties = self._parse_properties(paren_props_str)
        
        # Parse remaining content (properties or text)
        text_content = None
        if remaining:
            remaining = remaining.strip()
            if remaining.startswith('"') and remaining.endswith('"'):
                # Quoted text content
                text_content = remaining[1:-1]
            else:
                # More properties
                more_props = self._parse_properties(remaining)
                properties.update(more_props)
        
        # Create node
        node = UINode(
            tag=tag,
            id=node_id,
            mixins=mixins,
            properties=properties,
            text_content=text_content,
        )
        
        return node
    
    def _parse_properties(self, props_str: str) -> Dict[str, Any]:
        """
        Parse property string into dictionary.
        
        Properties are space-separated key=value pairs.
        Values can be:
        - Bare words/numbers (unquoted)
        - Literal strings (quoted)
        - References ($lambda:, mcp:, @)
        
        Args:
            props_str: Property string
            
        Returns:
            Dictionary of properties
        """
        props = {}
        
        # Simple tokenization
        tokens = self._tokenize(props_str)
        
        for token in tokens:
            if '=' in token:
                key, value = token.split('=', 1)
                # Remove quotes if present
                if value.startswith('"') and value.endswith('"'):
                    value = value[1:-1]
                props[key] = self._parse_value(value)
            else:
                # Bare word as property key with True value
                props[token] = True
        
        return props
    
    def _tokenize(self, s: str) -> List[str]:
        """
        Tokenize a string, respecting quoted values.
        
        Args:
            s: Input string
            
        Returns:
            List of tokens
        """
        tokens = []
        current = []
        in_quotes = False
        
        for char in s:
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
        
        return tokens
    
    def _parse_value(self, value: str) -> Any:
        """
        Parse a property value.
        
        Args:
            value: Value string
            
        Returns:
            Parsed value (Reference, number, bool, or string)
        """
        # Check for references
        if value.startswith(("$lambda:", "mcp:", "@")):
            return Reference.parse(value)
        
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
    
    def _get_indent(self, line: str) -> int:
        """
        Get indentation level of a line.
        
        Args:
            line: Line content
            
        Returns:
            Indentation level (number of spaces)
        """
        stripped = line.lstrip()
        return len(line) - len(stripped)
    
    def get_errors(self) -> List[str]:
        """Get parse errors."""
        return self._errors.copy()


def parse_auil(source: str) -> UINode:
    """
    Convenience function to parse AUIL source.
    
    Args:
        source: AUIL source code
        
    Returns:
        Root UINode of the parsed tree
        
    Example:
        >>> from auil_parser import parse_auil
        >>> tree = parse_auil('''
        ... stack#root dir=v gap=m
        ...   text(role=title) "Hello"
        ... ''')
    """
    parser = AUILParser()
    return parser.parse(source)
