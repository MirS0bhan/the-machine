"""
Cross-component integration tests: Agent Core → Lambda Server (`lambda.deprecate`).

Workflow:
    1. Agent Core registers a function via `lambda.register` (policy-gated).
    2. Agent Core marks an older version deprecated via `lambda.deprecate`.
    3. New invocations continue to use the latest non-deprecated version.

Assertions:
    - Deprecating an existing version returns success.
    - Deprecating a missing function or version returns failure.
    - Missing name/version arguments return an MCP error.
"""

from policy_broker.models import CheckRequest

ADD_V1 = """
def add(input):
    a = input.get("a", 0)
    b = input.get("b", 0)
    return {"sum": a + b, "version": 1}
"""

ADD_V2 = """
def add(input):
    a = input.get("a", 0)
    b = input.get("b", 0)
    return {"sum": a + b, "version": 2, "updated": True}
"""


def _gate_register(broker, func_name: str) -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_IPC_CALL",
            path=func_name,
            method="lambda.register",
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


def _gate_deprecate(broker, func_name: str) -> dict:
    resp = broker.check(
        CheckRequest(
            capability="CAP_IPC_CALL",
            path=func_name,
            method="lambda.deprecate",
            principal="agent-core",
            provenance="user-intent",
        )
    )
    return {"allowed": resp.decision == "ALLOW", "response": resp}


class TestLambdaDeprecatePolicyGated:
    """Register multiple versions, deprecate older ones, invoke latest."""

    def test_deprecate_existing_version(
        self,
        lambda_mcp,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "calc.*",
                    "method": "lambda.register",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
                {
                    "path": "calc.*",
                    "method": "lambda.deprecate",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
                {
                    "path": "calc.*",
                    "method": "lambda.invoke",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
            ]
        )

        assert _gate_register(policy_broker, "calc.add")["allowed"]

        r1 = lambda_mcp.handle_tool_call(
            "lambda.register",
            {
                "name": "calc.add",
                "runtime": "python3.12",
                "code": ADD_V1,
                "description": "Add two numbers v1",
                "input_schema": {},
                "output_schema": {},
                "capabilities": [],
            },
        )
        assert r1["manifest"]["version"] == 1

        r2 = lambda_mcp.handle_tool_call(
            "lambda.register",
            {
                "name": "calc.add",
                "runtime": "python3.12",
                "code": ADD_V2,
                "description": "Add two numbers v2",
                "input_schema": {},
                "output_schema": {},
                "capabilities": [],
            },
        )
        assert r2["manifest"]["version"] == 2

        assert _gate_deprecate(policy_broker, "calc.add")["allowed"]

        deprecate_result = lambda_mcp.handle_tool_call(
            "lambda.deprecate",
            {"name": "calc.add", "version": 1},
        )
        assert deprecate_result.get("success") is True

        invoke_result = lambda_mcp.handle_tool_call(
            "lambda.invoke",
            {"name": "calc.add", "input": {"a": 10, "b": 32}},
        )
        assert invoke_result.get("success") is True
        assert invoke_result["output"]["sum"] == 42
        assert invoke_result["output"]["updated"] is True

    def test_deprecate_unknown_version_returns_false(
        self,
        lambda_mcp,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "calc.*",
                    "method": "lambda.register",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
                {
                    "path": "calc.*",
                    "method": "lambda.deprecate",
                    "decision": "ALLOW",
                    "capabilities": ["CAP_IPC_CALL"],
                },
            ]
        )

        lambda_mcp.handle_tool_call(
            "lambda.register",
            {
                "name": "calc.single",
                "runtime": "python3.12",
                "code": "def single(input): return {'ok': True}",
                "description": "Single version",
                "input_schema": {},
                "output_schema": {},
                "capabilities": [],
            },
        )

        result = lambda_mcp.handle_tool_call(
            "lambda.deprecate",
            {"name": "calc.single", "version": 99},
        )
        assert result.get("success") is False

    def test_deprecate_unknown_function_returns_false(self, lambda_mcp):
        result = lambda_mcp.handle_tool_call(
            "lambda.deprecate",
            {"name": "calc.missing", "version": 1},
        )
        assert result.get("success") is False

    def test_deprecate_requires_name_and_version(self, lambda_mcp):
        missing_version = lambda_mcp.handle_tool_call(
            "lambda.deprecate",
            {"name": "calc.add"},
        )
        assert "error" in missing_version

        missing_name = lambda_mcp.handle_tool_call(
            "lambda.deprecate",
            {"version": 1},
        )
        assert "error" in missing_name

    def test_deny_deprecate_without_policy(
        self,
        policy_broker,
        register_policy,
    ):
        register_policy(
            [
                {
                    "path": "calc.*",
                    "method": "lambda.deprecate",
                    "decision": "DENY",
                    "capabilities": ["CAP_IPC_CALL"],
                },
            ]
        )

        gate = _gate_deprecate(policy_broker, "calc.add")
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"
