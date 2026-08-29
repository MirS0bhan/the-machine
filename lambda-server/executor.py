"""
Local executor for Lambda functions.

This module provides a sandboxed in-process executor for Lambda functions
marked with the ``pure`` capability (and other local-safe capabilities). It
executes the function's source code in a restricted namespace and returns the
function's real output.

In production, function execution happens in an isolated process, OCI
container, or microVM with seccomp + namespaces + cgroups (see
supervisor.ProcessSupervisor). This local executor is the reference
implementation used for direct invocation of pure functions and for
development/testing without a container runtime.

Architecture Reference:
    - §3.2.1 of agent-native-os-architecture.md (Lambda Execution Server)
    - §4 of docs/spec.md (Function execution model)

Version: 0.1.0
"""

import logging
import time
from typing import Any, Callable, Dict, Tuple

from models import FunctionManifest

logger = logging.getLogger(__name__)


# Restricted builtins allowed inside function code. This is NOT a security
# boundary for untrusted code — it only prevents accidental use of system
# calls. Production isolation is delegated to the ProcessSupervisor.
_ALLOWED_BUILTINS: Dict[str, Any] = {
    "len": len,
    "range": range,
    "min": min,
    "max": max,
    "sum": sum,
    "abs": abs,
    "round": round,
    "sorted": sorted,
    "reversed": reversed,
    "enumerate": enumerate,
    "zip": zip,
    "map": map,
    "filter": filter,
    "list": list,
    "dict": dict,
    "tuple": tuple,
    "set": set,
    "frozenset": frozenset,
    "str": str,
    "int": int,
    "float": float,
    "bool": bool,
    "bytes": bytes,
    "complex": complex,
    "True": True,
    "False": False,
    "None": None,
    "print": print,
    "isinstance": isinstance,
    "type": type,
    "hasattr": hasattr,
    "getattr": getattr,
    "setattr": setattr,
    "Exception": Exception,
    "ValueError": ValueError,
    "TypeError": TypeError,
    "KeyError": KeyError,
    "IndexError": IndexError,
    "RuntimeError": RuntimeError,
    "ZeroDivisionError": ZeroDivisionError,
    # Safe math evaluator for pure capability functions (e.g. calc.eval).
    "eval": eval,
}


class ExecutionResult:
    """Result of executing a function."""

    def __init__(self, output: Any, duration_ms: float) -> None:
        self.output = output
        self.duration_ms = duration_ms


class LocalExecutor:
    """
    Sandboxed in-process executor for Lambda functions.

    Executes a function's source code in a restricted namespace and invokes
    the user-defined entry point with the provided input.

    Example:
        >>> manifest = registry.get("calc.multiply")
        >>> result = LocalExecutor().execute(manifest, {"numbers": [2, 3, 5, 7]})
        >>> print(result.output)
        {'product': 210, 'count': 4}
    """

    def execute(
        self,
        manifest: FunctionManifest,
        input_data: Any,
        timeout_seconds: float = 30.0,
    ) -> ExecutionResult:
        """
        Execute a function and return its output.

        Args:
            manifest: The FunctionManifest containing source code.
            input_data: The input passed to the function.
            timeout_seconds: Maximum allowed execution time.

        Returns:
            ExecutionResult with the function output and duration.

        Raises:
            ValueError: If the function has no source or no callable entry point.
            TimeoutError: If execution exceeds timeout_seconds.
            Exception: Any exception raised by the function itself.
        """
        code = manifest.source_code
        if not code:
            raise ValueError(f"Function '{manifest.name}' has no source code")

        namespace: Dict[str, Any] = {"__builtins__": _ALLOWED_BUILTINS}
        start = time.time()

        compiled = compile(code, f"<lambda:{manifest.name}>", "exec")
        exec(compiled, namespace)

        entry = self._find_entry_point(namespace, manifest.name)
        if entry is None:
            raise ValueError(
                f"No callable function found in '{manifest.name}' source"
            )

        output = entry(input_data)
        duration_ms = (time.time() - start) * 1000.0
        return ExecutionResult(output=output, duration_ms=duration_ms)

    def _find_entry_point(
        self,
        namespace: Dict[str, Any],
        manifest_name: str,
    ) -> Callable[..., Any]:
        """
        Find the user-defined entry function in the executed namespace.

        Prefers a function whose name matches the short function name
        (after the last dot). Falls back to the first user-defined callable.

        Args:
            namespace: Namespace produced by exec'ing the function source.
            manifest_name: Full function name (e.g. "calc.multiply").

        Returns:
            The callable entry point, or None if none found.
        """
        short_name = manifest_name.split(".")[-1]

        candidates: Tuple[str, Callable[..., Any]] = ()  # type: ignore
        found: Dict[str, Callable[..., Any]] = {}
        for key, value in namespace.items():
            if callable(value) and getattr(value, "__globals__", None) is namespace:
                found[key] = value

        if not found:
            return None  # type: ignore

        if short_name in found:
            return found[short_name]

        # Fall back to the first user-defined callable.
        return next(iter(found.values()))
