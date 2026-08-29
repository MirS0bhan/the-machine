from functools import wraps
from typing import Callable, TypeVar, Any
from .policy_client import PolicyClient

T = TypeVar('T')

# Capabilities
CAP_STATE_READ = "CAP_STATE_READ"
CAP_STATE_WRITE = "CAP_STATE_WRITE"

policy_client = PolicyClient()


def policy_check(capability: str) -> Callable[[Callable[..., T]], Callable[..., T]]:
    """Decorator to check if the request has the required capability."""
    def decorator(f: Callable[..., T]) -> Callable[..., T]:
        @wraps(f)
        async def wrapper(*args: Any, **kwargs: Any) -> T:
            path = kwargs.get("path") or (args[0].path if args and hasattr(args[0], "path") else None)
            await policy_client.check(capability=capability, path=path)
            return await f(*args, **kwargs)
        return wrapper
    return decorator