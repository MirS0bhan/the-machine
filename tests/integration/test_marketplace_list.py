"""
Cross-component integration tests: Agent Core → Marketplace (policy-gated).

Workflow:
    1. Agent Core lists available capability bundles via `marketplace.list`.
    2. Policy Broker gates the call using `CAP_IPC_CALL`.
    3. Marketplace catalog returns seeded bundles (calc-pack-v1 on production hosts).

Assertions:
    - Policy Broker allows `marketplace.list` for agent-core when permitted.
    - Catalog exposes the built-in calculator pack metadata.
    - Policy Broker denies listing when no matching rule exists.
"""

from dataclasses import dataclass
from typing import List

import pytest
from policy_broker.models import CheckRequest


@dataclass(frozen=True)
class CapabilityBundle:
    id: str
    name: str
    version: str
    description: str


class MarketplaceCatalog:
    """In-process stub mirroring marketplace/src/main.rs seed data."""

    def __init__(self) -> None:
        self._bundles = {
            "calc-pack-v1": CapabilityBundle(
                id="calc-pack-v1",
                name="Calculator Pack",
                version="1.0.0",
                description="Basic calculator lambda + UI button",
            ),
        }

    def list_bundles(self) -> List[CapabilityBundle]:
        return list(self._bundles.values())


def _gate_marketplace_list(broker, method: str = "marketplace.list") -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_IPC_CALL",
            path="marketplace.list",
            method=method,
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


@pytest.fixture
def marketplace_catalog() -> MarketplaceCatalog:
    return MarketplaceCatalog()


class TestMarketplaceListIntegration:
    def test_list_allowed_and_returns_calc_pack(
        self,
        policy_broker,
        register_policy,
        marketplace_catalog,
    ):
        register_policy(
            [
                {
                    "path": "marketplace.*",
                    "method": "marketplace.list",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
            ]
        )

        gate = _gate_marketplace_list(policy_broker)
        assert gate["allowed"], f"Broker DENY: {gate['response'].message}"

        bundles = marketplace_catalog.list_bundles()
        assert len(bundles) >= 1
        calc = next(b for b in bundles if b.id == "calc-pack-v1")
        assert calc.name == "Calculator Pack"
        assert calc.version == "1.0.0"
        assert "calculator" in calc.description.lower()

    def test_list_denied_without_policy(
        self,
        policy_broker,
        marketplace_catalog,
    ):
        gate = _gate_marketplace_list(policy_broker)
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

        # Catalog still readable in-process; production path is broker-gated first.
        assert any(b.id == "calc-pack-v1" for b in marketplace_catalog.list_bundles())
