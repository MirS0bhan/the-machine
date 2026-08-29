"""
ASL (Agent Style Language) Parser.

This module implements the parser for ASL, a style and motion language
for the UI engine. ASL defines design tokens, style mixins, and
state-based animations.

Architecture Reference:
    - §3 of docs/spec.md (ASL — style & motion language)

Version: 0.1.0
"""

import re
import logging
from typing import Any, Dict, List, Optional, Tuple, Union
from models import (
    StyleMixin,
    StateTransition,
    DesignToken,
    AdaptiveColor,
    MotionCurve,
    Scale,
    EventType,
)

logger = logging.getLogger(__name__)


# Regex patterns
TOKEN_PATTERN = re.compile(
    r'^token\s+([\w.]+)\s*=\s*(.+)$'
)
SCALE_PATTERN = re.compile(
    r'^scale\s+(\w+):\s*(.+)$'
)
MOTION_PATTERN = re.compile(
    r'^motion\s+(\w+)\s*=\s*(.+)$'
)
STYLE_PATTERN = re.compile(
    r'^style\s+(\w+)$'
)
STATE_PATTERN = re.compile(
    r'^state:(\w+)\s*→\s*(.+)$'
)
PROPERTY_PATTERN = re.compile(
    r'^([\w:]+)\s*=\s*(.+)$'
)
TRANSITION_PATTERN = re.compile(
    r'^on:(\w+)\s*=>\s*(.+)$'
)


class ASLParser:
    """
    Parser for ASL (Agent Style Language).
    
    ASL defines:
    - Design tokens (colors, spacing, elevation, motion)
    - Scale definitions
    - Motion curves
    - Style mixins with state transitions
    
    Example:
        >>> parser = ASLParser()
        >>> result = parser.parse('''
        ... token surface.primary = adaptive(light:#FFFFFFEE dark:#1E1E1EEE)
        ... token accent = system.accent
        ... 
        ... scale radius: sm=6 md=10 lg=16
        ... 
        ... motion snappy = spring(stiffness=300 damping=26)
        ... 
        ... style Surface
        ...   bg=token:surface.primary
        ...   radius=r-lg
        ... 
        ... style Hoverable
        ...   on:hover => elev=e2 scale=1.02 motion=snappy
        ... ''')
    """
    
    def __init__(self):
        """Initialize the parser."""
        self.tokens: Dict[str, DesignToken] = {}
        self.scales: Dict[str, Scale] = {}
        self.motions: Dict[str, MotionCurve] = {}
        self.styles: Dict[str, StyleMixin] = {}
        self._errors: List[str] = []
        self._line_number = 0
    
    def parse(self, source: str) -> Dict[str, Any]:
        """
        Parse ASL source.
        
        Args:
            source: ASL source code
            
        Returns:
            Dictionary with parsed tokens, scales, motions, and styles
        """
        lines = source.split('\n')
        self._line_number = 0
        self._errors = []
        
        current_style: Optional[StyleMixin] = None
        
        for line in lines:
            self._line_number += 1
            line = line.strip()
            
            if not line or line.startswith('//'):
                continue
            
            # Try to match token definition
            match = TOKEN_PATTERN.match(line)
            if match:
                name = match.group(1)
                value_str = match.group(2)
                token = self._parse_token(name, value_str)
                if token:
                    self.tokens[name] = token
                continue
            
            # Try to match scale definition
            match = SCALE_PATTERN.match(line)
            if match:
                name = match.group(1)
                values_str = match.group(2)
                scale = self._parse_scale(name, values_str)
                if scale:
                    self.scales[name] = scale
                continue
            
            # Try to match motion definition
            match = MOTION_PATTERN.match(line)
            if match:
                name = match.group(1)
                value_str = match.group(2)
                motion = self._parse_motion(name, value_str)
                if motion:
                    self.motions[name] = motion
                continue
            
            # Try to match style definition
            match = STYLE_PATTERN.match(line)
            if match:
                name = match.group(1)
                current_style = StyleMixin(name=name)
                self.styles[name] = current_style
                continue
            
            # Try to match state transition (inside style)
            match = STATE_PATTERN.match(line)
            if match and current_style:
                state = match.group(1)
                props_str = match.group(2)
                transition = self._parse_state_transition(state, props_str)
                if transition:
                    current_style.transitions[state] = transition
                continue
            
            # Try to match transition (inside style)
            match = TRANSITION_PATTERN.match(line)
            if match and current_style:
                event = match.group(1)
                props_str = match.group(2)
                transition = self._parse_event_transition(event, props_str)
                if transition:
                    current_style.transitions[event] = transition
                continue
            
            # Try to match property (inside style)
            match = PROPERTY_PATTERN.match(line)
            if match and current_style:
                key = match.group(1)
                value = match.group(2)
                current_style.properties[key] = self._parse_value(value)
                continue
        
        if self._errors:
            logger.warning(f"ASL parse completed with {len(self._errors)} warnings")
        
        return {
            "tokens": self.tokens,
            "scales": self.scales,
            "motions": self.motions,
            "styles": self.styles,
        }
    
    def _parse_token(self, name: str, value_str: str) -> Optional[DesignToken]:
        """
        Parse a token definition.
        
        Args:
            name: Token name
            value_str: Value string
            
        Returns:
            DesignToken or None if parsing failed
        """
        # Check for adaptive color
        adaptive_match = re.match(
            r'adaptive\(light:(\S+)\s+dark:(\S+)\)',
            value_str
        )
        if adaptive_match:
            light = adaptive_match.group(1)
            dark = adaptive_match.group(2)
            value = AdaptiveColor(light=light, dark=dark)
        elif value_str.startswith('system.'):
            # System token reference
            value = value_str
        else:
            value = value_str
        
        return DesignToken(name=name, value=value)
    
    def _parse_scale(self, name: str, values_str: str) -> Optional[Scale]:
        """
        Parse a scale definition.
        
        Args:
            name: Scale name
            values_str: Values string (e.g., "sm=6 md=10 lg=16")
            
        Returns:
            Scale or None if parsing failed
        """
        values = {}
        for part in values_str.split():
            if '=' in part:
                key, value = part.split('=', 1)
                try:
                    values[key] = int(value)
                except ValueError:
                    try:
                        values[key] = float(value)
                    except ValueError:
                        values[key] = value
        
        return Scale(name=name, values=values)
    
    def _parse_motion(self, name: str, value_str: str) -> Optional[MotionCurve]:
        """
        Parse a motion definition.
        
        Args:
            name: Motion name
            value_str: Value string
            
        Returns:
            MotionCurve or None if parsing failed
        """
        # Check for spring
        spring_match = re.match(
            r'spring\(stiffness=(\d+(?:\.\d+)?)\s+damping=(\d+(?:\.\d+)?)\)',
            value_str
        )
        if spring_match:
            return MotionCurve(
                name=name,
                type="spring",
                stiffness=float(spring_match.group(1)),
                damping=float(spring_match.group(2)),
            )
        
        # Check for duration
        duration_match = re.match(
            r'duration\((\d+)ms\s+(?:ease=)?([\w-]+)\)',
            value_str
        )
        if duration_match:
            return MotionCurve(
                name=name,
                type="duration",
                duration_ms=float(duration_match.group(1)),
                easing=duration_match.group(2),
            )
        
        self._errors.append(f"Line {self._line_number}: Invalid motion: {value_str}")
        return None
    
    def _parse_state_transition(self, state: str, props_str: str) -> Optional[StateTransition]:
        """
        Parse a state transition.
        
        Args:
            state: Target state name
            props_str: Properties string
            
        Returns:
            StateTransition or None if parsing failed
        """
        properties = {}
        motion = None
        
        # Parse properties
        for part in props_str.split():
            if '=' in part:
                key, value = part.split('=', 1)
                if key == 'motion':
                    motion = value
                else:
                    properties[key] = self._parse_value(value)
        
        return StateTransition(
            state=state,
            properties=properties,
            motion=motion,
        )
    
    def _parse_event_transition(self, event: str, props_str: str) -> Optional[StateTransition]:
        """
        Parse an event-based transition.
        
        Args:
            event: Event name (e.g., "hover", "press")
            props_str: Properties string
            
        Returns:
            StateTransition or None if parsing failed
        """
        properties = {}
        motion = None
        
        # Parse properties
        for part in props_str.split():
            if '=' in part:
                key, value = part.split('=', 1)
                if key == 'motion':
                    motion = value
                else:
                    properties[key] = self._parse_value(value)
        
        return StateTransition(
            state=event,
            properties=properties,
            motion=motion,
        )
    
    def _parse_value(self, value: str) -> Any:
        """
        Parse a value string.
        
        Args:
            value: Value string
            
        Returns:
            Parsed value
        """
        # Check for token reference
        if value.startswith('token:'):
            return {"type": "token_ref", "name": value[6:]}
        
        # Check for scale reference
        if value.startswith('r-') or value.startswith('s-') or value.startswith('e-'):
            return {"type": "scale_ref", "value": value}
        
        # Try to parse as number
        try:
            if '.' in value:
                return float(value)
            else:
                return int(value)
        except ValueError:
            pass
        
        # String value
        return value
    
    def get_errors(self) -> List[str]:
        """Get parse errors."""
        return self._errors.copy()


def parse_asl(source: str) -> Dict[str, Any]:
    """
    Convenience function to parse ASL source.
    
    Args:
        source: ASL source code
        
    Returns:
        Dictionary with parsed tokens, scales, motions, and styles
        
    Example:
        >>> from asl_parser import parse_asl
        >>> result = parse_asl('''
        ... token surface.primary = adaptive(light:#FFFFFFEE dark:#1E1E1EEE)
        ... style Surface
        ...   bg=token:surface.primary
        ... ''')
    """
    parser = ASLParser()
    return parser.parse(source)
