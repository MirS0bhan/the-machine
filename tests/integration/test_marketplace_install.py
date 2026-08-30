"""
Cross-component integration tests: Agent Core → Marketplace (`marketplace.install`).

Workflow:
    1. Agent Core requests installation of a capability bundle via `marketplace.install`.
    2. Policy Broker gates the call using `CAP_IPC_CALL`.
    3. Marketplace requires human confirmation (`confirm: true`) when `CONFIRM_REQUIRED`.
    4. Upon confirmation and signature verification, bundle lambdas are registered
       and UI patches are applied to the UI runtime.
    5. Installed bundle ID is recorded in the marketplace registry.

Assertions:
    - Policy Broker allows `marketplace.install` when permitted by policy rules.
    - Policy Broker denies `marketplace.install` when no policy rule matches.
    - Unconfirmed installation returns status `CONFIRM_REQUIRED`.
    - Confirmed installation succeeds and registers bundle lambdas in the Lambda Server.
    - Installing a non-existent bundle returns failure / not-found.
    - Forged or invalid signatures fail bundle installation.
"""

from dataclasses import dataclass, field
import hashlib
from typing import Any, Dict, List, Optional

import pytest
from policy_broker.models import CheckRequest


def sign_bundle(bundle_id: str) -> str:
    h = hashlib.sha256()
    h.update(bundle_id.encode("utf-8"))
    h.update(b"the-machine-marketplace-v1")
    return h.hexdigest()


@dataclass
class CapabilityBundle:
    id: str
    name: str
    version: str
    description: str
    signature: str
    lambdas: List[Dict[str, Any]] = field(default_factory=list)
    ui_patches: List[Dict[str, Any]] = field(default_factory=list)
    policy_rules: List[Dict[str, Any]] = field(default_factory=list)


class MarketplaceCatalog:
    """In-process marketplace catalog mirroring marketplace/src/main.rs install flow."""

    def __init__(self) -> None:
        calc_id = "calc-pack-v1"
        self._bundles: Dict[str, CapabilityBundle] = {
            calc_id: CapabilityBundle(
                id=calc_id,
                name="Calculator Pack",
                version="1.0.0",
                description="Basic calculator lambda + UI button",
                signature=sign_bundle(calc_id),
                lambdas=[
                    {
                        "name": "calc.eval",
                        "description": "Evaluate math expressions",
                        "runtime": "python3.12",
                        "code": "def eval_expr(input): return {'result': 42}",
                        "input_schema": {"expression": "string"},
                        "output_schema": {"result": "number"},
                        "capabilities": [],
                        "exposes_mcp": "calc.*",
                    }
                ],
                ui_patches=[
                    {
                        "op": "insert",
                        "anchor": "ui.root",
                        "node": {
                            "id": "ui.calc_btn",
                            "type": "button",
                            "props": {"label": "Calculate"},
                        },
                    }
                ],
                policy_rules=[
                    {"capability": "CAP_IPC_CALL", "path": "calc.*", "decision": "ALLOW"}
                ],
            ),
        }
        self.installed: List[str] = []

    def verify_bundle(self, bundle: CapabilityBundle) -> bool:
        return bool(bundle.signature) and bundle.signature == sign_bundle(bundle.id)

    def install(
        self,
        bundle_id: str,
        confirm: bool = False,
        lambda_mcp=None,
    ) -> Dict[str, Any]:
        bundle = self._bundles.get(bundle_id)
        if not bundle:
            return {"error": "E_NOT_FOUND", "message": "bundle not found"}

        if not confirm:
            return {
                "status": "CONFIRM_REQUIRED",
                "bundle": bundle.name,
            }

        if not self.verify_bundle(bundle):
            return {
                "error": "E_SIGNATURE",
                "message": "bundle signature invalid",
            }

        if lambda_mcp:
            for l in bundle.lambdas:
                lambda_mcp.handle_tool_call(
                    "lambda.register",
                    {
                        "name": l["name"],
                        "runtime": l.get("runtime", "python3.12"),
                        "code": l["code"],
                        "description": l.get("description", ""),
                        "input_schema": l.get("input_schema", {}),
                        "output_schema": l.get("output_schema", {}),
                        "capabilities": l.get("capabilities", []),
                        "exposes_mcp": l.get("exposes_mcp"),
                    },
                )

        if bundle_id not in self.installed:
            self.installed.append(bundle_id)

        return {
            "status": "ok",
            "installed": bundle_id,
        }


def _gate_marketplace_install(broker, method: str = "marketplace.install") -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_IPC_CALL",
            path="marketplace.install",
            method=method,
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


@pytest.fixture
def marketplace_catalog() -> MarketplaceCatalog:
    return MarketplaceCatalog()


class TestMarketplaceInstallIntegration:
    """End-to-end: gate-check → confirm → signature-verify → lambda register → installed."""

    def test_install_requires_confirmation_first(
        self,
        policy_broker,
        register_policy,
        marketplace_catalog,
    ):
        register_policy(
            [
                {
                    "path": "marketplace.*",
                    "method": "marketplace.install",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
            ]
        )

        gate = _gate_marketplace_install(policy_broker)
        assert gate["allowed"], f"Broker DENY: {gate['response'].message}"

        res = marketplace_catalog.install("calc-pack-v1", confirm=False)
        assert res.get("status") == "CONFIRM_REQUIRED"
        assert res.get("bundle") == "Calculator Pack"
        assert "calc-pack-v1" not in marketplace_catalog.installed

    def test_install_confirmed_registers_lambdas_and_records_installed(
        self,
        policy_broker,
        register_policy,
        marketplace_catalog,
        lambda_mcp,
    ):
        register_policy(
            [
                {
                    "path": "marketplace.*",
                    "method": "marketplace.install",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
            ]
        )

        gate = _gate_marketplace_install(policy_broker)
        assert gate["allowed"]

        res = marketplace_catalog.install("calc-pack-v1", confirm=True, lambda_mcp=lambda_mcp)
        assert res.get("status") == "ok"
        assert res.get("installed") == "calc-pack-v1"
        assert "calc-pack-v1" in marketplace_catalog.installed

        # Ensure registered lambda is discoverable via lambda.search
        search_res = lambda_mcp.handle_tool_call("lambda.search", {"query": "math"})
        assert search_res.get("count", 0) >= 1
        names = [item["name"] for item in search_res["results"]]
        assert "calc.eval" in names

    def test_install_denied_without_policy(
        self,
        policy_broker,
        marketplace_catalog,
    ):
        gate = _gate_marketplace_install(policy_broker)
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

    def test_install_unknown_bundle_returns_not_found(
        self,
        policy_broker,
        register_policy,
        marketplace_catalog,
    ):
        register_policy(
            [
                {
                    "path": "marketplace.*",
                    "method": "marketplace.install",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
            ]
        )

        gate = _gate_marketplace_install(policy_broker)
        assert gate["allowed"]

        res = marketplace_catalog.install("nonexistent-bundle", confirm=True)
        assert res.get("error") == "E_NOT_FOUND"

    def test_install_rejects_invalid_signature(
        self,
        policy_broker,
        register_policy,
        marketplace_catalog,
    ):
        register_policy(
            [
                {
                    "path": "marketplace.*",
                    "method": "marketplace.install",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
            ]
        )

        # Register a tampered bundle in the catalog
        tampered_bundle = CapabilityBundle(
            id="tampered-pack",
            name="Tampered Pack",
            version="1.0.0",
            description="Tampered description",
            signature="deadbeef0123456789abcdef",
        )
        marketplace_catalog._bundles["tampered-pack"] = tampered_bundle

        gate = _gate_marketplace_install(policy_broker)
        assert gate["allowed"]

        res = marketplace_catalog.install("tampered-pack", confirm=True)
        assert res.get("error") == "E_SIGNATURE"
        assert "tampered-pack" not in marketplace_catalog.installed
