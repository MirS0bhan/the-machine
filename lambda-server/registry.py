"""
Function Registry for the Lambda Execution Server.

This module implements the Function Registry, which stores function metadata
with full version history. The Registry is the source of truth for all
registered functions and provides search, versioning, and lifecycle management.

Key Features:
    - Named, described, persistent, reusable functions
    - Immutable version history (each registration creates a new version)
    - Keyword-based search over function descriptions
    - MCP pattern resolution for UI-bound functions
    - Rollback to previous versions

Architecture Reference:
    - §3.2.1 of agent-native-os-architecture.md (Lambda Execution Server)
    - §6 of docs/spec.md (Function Registry — entry schema)

Version: 0.1.0
"""

import logging
import time
from typing import Any, Dict, List, Optional, Set, Tuple

from models import (
    CapabilityGrant,
    CapabilityPreset,
    FunctionManifest,
    FunctionVersion,
)

logger = logging.getLogger(__name__)


class FunctionRegistry:
    """
    Registry for named, described, persistent, reusable functions.
    
    The FunctionRegistry is the central storage for all function metadata.
    It maintains full version history and provides search capabilities
    for the agent to discover existing functions.
    
    Design Principles:
        1. Functions are named, described, persistent, reusable
        2. Each registration creates a new immutable version
        3. Search returns ranked metadata for the agent to choose from
        4. MCP patterns allow UI-bound functions to be invoked directly
    
    Example:
        >>> registry = FunctionRegistry()
        >>> manifest = registry.register(
        ...     name="calc.add",
        ...     runtime="python3.12",
        ...     code="def add(input): return {'sum': sum(input['values'])}",
        ...     description="Adds two or more numeric values",
        ...     input_schema={"values": "number[]"},
        ...     output_schema={"sum": "number"},
        ...     capabilities=set(),
        ... )
        >>> results = registry.search("add")
    """
    
    def __init__(self) -> None:
        """
        Initialize the FunctionRegistry.
        
        Creates empty storage for functions and MCP exposures.
        """
        # function_name -> list of FunctionVersion (ordered by version)
        self._functions: Dict[str, List[FunctionVersion]] = {}
        # MCP-exposed patterns -> function_name (e.g., "calc.*" -> "calc.eval")
        self._mcp_exposures: Dict[str, str] = {}
        
        logger.info("FunctionRegistry initialized")
    
    def register(
        self,
        name: str,
        runtime: str,
        code: str,
        description: str,
        input_schema: Dict[str, Any],
        output_schema: Dict[str, Any],
        capabilities: Set[CapabilityGrant],
        exposes_mcp: Optional[str] = None,
    ) -> FunctionManifest:
        """
        Create or update a function. Each call creates a new immutable version.
        
        This is the primary method for registering functions. The Registry
        validates that the function name is unique (or creates a new version
        if it already exists).
        
        Args:
            name: Unique function name (e.g., "calc.add")
            runtime: Language runtime (e.g., "python3.12", "node18", "go1.21")
            code: Source code for the function
            description: Human-readable description of what the function does
            input_schema: JSON schema for input validation
            output_schema: JSON schema for output validation
            capabilities: Set of capability grants for this function
            exposes_mcp: Optional MCP pattern to expose (e.g., "calc.*")
            
        Returns:
            The newly created FunctionManifest with version number
            
        Raises:
            ValueError: If required parameters are missing or invalid
            
        Example:
            >>> manifest = registry.register(
            ...     name="calc.add",
            ...     runtime="python3.12",
            ...     code="def add(input): return {'sum': sum(input['values'])}",
            ...     description="Adds two or more numeric values",
            ...     input_schema={"values": "number[]"},
            ...     output_schema={"sum": "number"},
            ...     capabilities=set(),
            ... )
        """
        # Determine version number
        if name in self._functions:
            existing = self._functions[name]
            version = len(existing) + 1
            logger.info(f"Registering new version {version} for function '{name}'")
        else:
            version = 1
            self._functions[name] = []
            logger.info(f"Registering new function '{name}' (version 1)")
        
        # Create manifest
        manifest = FunctionManifest(
            name=name,
            version=version,
            runtime=runtime,
            description=description,
            input_schema=input_schema,
            output_schema=output_schema,
            capabilities=capabilities,
            source_code=code,
            exposes_mcp=exposes_mcp,
        )
        
        # Store version
        func_version = FunctionVersion(version=version, manifest=manifest)
        self._functions[name].append(func_version)
        
        # Register MCP exposure if specified
        if exposes_mcp:
            self._mcp_exposures[exposes_mcp] = name
            logger.info(f"Function '{name}' exposed as MCP pattern: {exposes_mcp}")
        
        logger.info(
            f"Function '{name}' registered successfully: "
            f"version={version}, runtime={runtime}, "
            f"capabilities={len(capabilities)}"
        )
        
        return manifest
    
    def get(self, name: str, version: Optional[int] = None) -> Optional[FunctionManifest]:
        """
        Get a function manifest by name and optional version.
        
        Args:
            name: Function name to retrieve
            version: Specific version number, or None for latest
            
        Returns:
            FunctionManifest if found, None otherwise
            
        Example:
            >>> manifest = registry.get("calc.add")  # Latest version
            >>> manifest = registry.get("calc.add", version=2)  # Specific version
        """
        if name not in self._functions:
            logger.debug(f"Function '{name}' not found in registry")
            return None
        
        versions = self._functions[name]
        if not versions:
            return None
        
        if version is None:
            return versions[-1].manifest
        
        for v in versions:
            if v.version == version:
                return v.manifest
        
        logger.debug(f"Version {version} not found for function '{name}'")
        return None
    
    def get_version_history(self, name: str) -> List[Dict[str, Any]]:
        """
        Get version history for a function.
        
        Args:
            name: Function name
            
        Returns:
            List of version metadata dictionaries
            
        Example:
            >>> history = registry.get_version_history("calc.add")
            >>> for v in history:
            ...     print(f"Version {v['version']}: created at {v['created_at']}")
        """
        if name not in self._functions:
            return []
        
        return [
            {
                "version": v.version,
                "created_at": v.created_at,
                "is_current": v == self._functions[name][-1],
            }
            for v in self._functions[name]
        ]
    
    def search(self, query: str) -> List[Dict[str, Any]]:
        """
        Semantic/keyword search over registry descriptions.
        
        Returns candidates with name, description, and schemas. This is the
        "is there already a function for this" step that the agent performs
        before writing new code.
        
        Currently implements keyword matching. Future versions could use
        embedding-based semantic search for better results.
        
        Args:
            query: Search query (keyword or semantic)
            
        Returns:
            List of matching function metadata dictionaries, sorted by relevance
            
        Example:
            >>> results = registry.search("calculate sum")
            >>> for r in results:
            ...     print(f"{r['name']}: {r['description']}")
        """
        query_lower = query.lower()
        results: List[Dict[str, Any]] = []
        
        for name, versions in self._functions.items():
            if not versions:
                continue
            
            latest = versions[-1].manifest
            # Simple keyword matching (could be enhanced with embeddings)
            if (query_lower in latest.description.lower() or
                query_lower in name.lower()):
                results.append({
                    "name": latest.name,
                    "description": latest.description,
                    "input_schema": latest.input_schema,
                    "output_schema": latest.output_schema,
                    "version": latest.version,
                    "runtime": latest.runtime,
                })
        
        # Sort by relevance (currently just name match priority)
        results.sort(key=lambda x: query_lower in x["name"].lower(), reverse=True)
        
        logger.debug(f"Search for '{query}' returned {len(results)} results")
        return results
    
    def list_calls(self, name: str) -> List[Dict[str, Any]]:
        """
        Introspect a function's declared CAP_IPC_CALL graph.
        
        This lets a human auditor or the Broker answer "what can this thing
        talk to" without reading the code.
        
        Args:
            name: Function name to introspect
            
        Returns:
            List of target functions this function can call
            
        Example:
            >>> calls = registry.list_calls("orchestrator")
            >>> for c in calls:
            ...     print(f"Can call: {c['target']} - {c['target_description']}")
        """
        manifest = self.get(name)
        if not manifest:
            return []
        
        ipc_calls = [c for c in manifest.capabilities if c.capability.name == "IPC_CALL"]
        
        result: List[Dict[str, Any]] = []
        for grant in ipc_calls:
            if grant.targets:
                for target in grant.targets:
                    # Try to get target info
                    target_manifest = self.get(target)
                    result.append({
                        "target": target,
                        "target_description": target_manifest.description if target_manifest else "unknown",
                    })
        
        return result
    
    def deprecate(self, name: str, version: int) -> bool:
        """
        Mark a specific version as deprecated.
        
        Deprecated versions are not used for new invocations but are
        retained for rollback purposes.
        
        Args:
            name: Function name
            version: Version number to deprecate
            
        Returns:
            True if successful, False if version not found
            
        Example:
            >>> success = registry.deprecate("calc.add", version=2)
        """
        if name not in self._functions:
            logger.warning(f"Cannot deprecate: function '{name}' not found")
            return False
        
        for v in self._functions[name]:
            if v.version == version:
                # In a full implementation, this would update status
                logger.info(f"Deprecated version {version} of function '{name}'")
                return True
        
        logger.warning(f"Cannot deprecate: version {version} not found for '{name}'")
        return False
    
    def rollback(self, name: str, target_version: int) -> Optional[FunctionManifest]:
        """
        Rollback to a previous version by creating a new version with the old code.
        
        This creates a new version (not reverts to an old one) to maintain
        the immutable version history.
        
        Args:
            name: Function name
            target_version: Version number to rollback to
            
        Returns:
            The new FunctionManifest if successful, None otherwise
            
        Example:
            >>> manifest = registry.rollback("calc.add", target_version=2)
            >>> print(f"Rolled back to version {manifest.version}")
        """
        if name not in self._functions:
            logger.warning(f"Cannot rollback: function '{name}' not found")
            return None
        
        # Find the target version
        target = self.get(name, target_version)
        if not target:
            logger.warning(f"Cannot rollback: version {target_version} not found for '{name}'")
            return None
        
        # Create new version with the old code
        logger.info(f"Rolling back function '{name}' to version {target_version}")
        return self.register(
            name=name,
            runtime=target.runtime,
            code=target.source_code,
            description=target.description,
            input_schema=target.input_schema,
            output_schema=target.output_schema,
            capabilities=target.capabilities,
            exposes_mcp=target.exposes_mcp,
        )
    
    def resolve_mcp_pattern(self, pattern: str) -> Optional[str]:
        """
        Resolve an MCP pattern to a function name.
        
        MCP patterns allow UI-bound functions to be invoked directly
        without the search step. For example, a calculator button's
        "on:press=mcp:calc.add" routes straight to the function.
        
        Args:
            pattern: MCP pattern to resolve (e.g., "calc.add", "calc.*")
            
        Returns:
            Function name if resolved, None otherwise
            
        Example:
            >>> func_name = registry.resolve_mcp_pattern("calc.add")
        """
        # Exact match
        if pattern in self._mcp_exposures:
            return self._mcp_exposures[pattern]
        
        # Wildcard match
        for exposed_pattern, func_name in self._mcp_exposures.items():
            if exposed_pattern.endswith("*"):
                prefix = exposed_pattern[:-1]
                if pattern.startswith(prefix):
                    return func_name
        
        return None
    
    def list_functions(self) -> List[Dict[str, Any]]:
        """
        List all registered functions.
        
        Returns:
            List of function metadata dictionaries
            
        Example:
            >>> functions = registry.list_functions()
            >>> for f in functions:
            ...     print(f"{f['name']}: {f['description']}")
        """
        results: List[Dict[str, Any]] = []
        for name, versions in self._functions.items():
            if versions:
                latest = versions[-1].manifest
                results.append({
                    "name": name,
                    "description": latest.description,
                    "version": latest.version,
                    "runtime": latest.runtime,
                    "status": latest.status,
                })
        return results
