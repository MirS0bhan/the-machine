"""
Cross-component integration tests: Policy Broker → State Store (`policy.register`).

Workflow:
    1. Operator registers a policy document via `policy.register`.
    2. Policy Broker persists the document to the State Store.
    3. Subsequent `policy.check` calls evaluate the registered rules.
    4. Re-registering replaces rules and changes check outcomes.

Assertions:
    - Registered policies are retrievable from the State Store.
    - ALLOW rules permit matching `policy.check` requests.
    - Updated DENY rules block previously allowed paths.
    - Default-deny applies when no rule matches.
"""

from policy_broker.models import CheckRequest, PolicyDoc, Rule


def _check(broker, *, path: str, method: str, capability: str = "CAP_IPC_CALL") -> str:
    resp = broker.check(
        CheckRequest(
            capability=capability,
            path=path,
            method=method,
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return resp.decision


class TestPolicyRegisterIntegration:
    """End-to-end: register policy → persist → check → re-register → assert."""

    def test_register_persists_policy_to_state_store(
        self,
        policy_broker,
        state_store,
    ):
        doc = PolicyDoc(
            rules=[
                Rule(
                    path="task.*",
                    method="state.set",
                    decision="ALLOW",
                    capabilities=["CAP_STATE_WRITE"],
                ),
            ]
        )
        policy_broker.register(doc, policy_key="ops")

        stored = state_store.get_policy("ops")
        assert stored is not None
        assert len(stored.rules) == 1
        assert stored.rules[0].path == "task.*"
        assert stored.rules[0].decision == "ALLOW"

    def test_registered_allow_rule_permits_matching_check(
        self,
        policy_broker,
    ):
        policy_broker.register(
            PolicyDoc(
                rules=[
                    Rule(
                        path="calc.*",
                        method="lambda.register",
                        decision="ALLOW",
                        capabilities=["CAP_IPC_CALL"],
                    ),
                ]
            )
        )

        assert _check(policy_broker, path="calc.add", method="lambda.register") == "ALLOW"

    def test_re_register_denies_previously_allowed_path(
        self,
        policy_broker,
    ):
        allow_doc = PolicyDoc(
            rules=[
                Rule(
                    path="calc.*",
                    method="lambda.register",
                    decision="ALLOW",
                    capabilities=["CAP_IPC_CALL"],
                ),
            ]
        )
        deny_doc = PolicyDoc(
            rules=[
                Rule(
                    path="calc.*",
                    method="lambda.register",
                    decision="DENY",
                    capabilities=["CAP_IPC_CALL"],
                ),
            ]
        )

        policy_broker.register(allow_doc)
        assert _check(policy_broker, path="calc.multiply", method="lambda.register") == "ALLOW"

        policy_broker.register(deny_doc)
        assert _check(policy_broker, path="calc.multiply", method="lambda.register") == "DENY"

    def test_default_deny_without_matching_rule(
        self,
        policy_broker,
    ):
        policy_broker.register(
            PolicyDoc(
                rules=[
                    Rule(
                        path="ui.task.*",
                        method="state.patch",
                        decision="ALLOW",
                        capabilities=["CAP_STATE_WRITE"],
                    ),
                ]
            )
        )

        assert _check(
            policy_broker,
            path="calc.add",
            method="lambda.register",
            capability="CAP_IPC_CALL",
        ) == "DENY"
