import httpx
from typing import Optional
from policy_broker.models import CheckResponse


class PolicyClient:
    def __init__(self, base_url: str = "http://localhost:8002"):
        self.base_url = base_url
        self.client = httpx.AsyncClient()

    async def check(self, capability: str, path: Optional[str] = None, principal: Optional[str] = None) -> CheckResponse:
        """Check if the principal has the required capability for the given path."""
        request = {
            "capability": capability,
            "path": path,
            "principal": principal
        }
        response = await self.client.post(f"{self.base_url}/policy/check", json=request)
        response.raise_for_status()
        return CheckResponse(**response.json())

    async def close(self):
        await self.client.aclose()