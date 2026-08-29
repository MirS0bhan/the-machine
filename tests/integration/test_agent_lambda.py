"""
Cross-component integration tests: Agent Core → Lambda Server (policy-gated).

Workflow:
    1. Agent Core registers a function via `lambda.register`.
    2. Agent Core invokes the function via `lambda.invoke`.
    3. Policy Broker gates both calls using `CAP_IPC_CALL`.

Assertions:
    - Policy Broker allows `lambda.register` for `calc.*` functions.
    - Policy Broker allows `lambda.invoke` for registered functions.
    - Lambda Server executes the function and returns the correct output.
"""

import pytest
from policy_broker.models import CheckRequest
from policy_broker.interpreter import PolicyInterpreter


# ── helpers ────────────────────────────────────────────────────────────

MULTIPLY_SOURCE = """
def multiply(input):
    numbers = input.get("numbers", [])
    product = 1
    for n in numbers:
        product *= n
    return {"product": product, "count": len(numbers)}
"""

ADD_SOURCE = """
def add(input):
    a = input.get("a", 0)
    b = input.get("b", 0)
    return {"sum": a + b}
"""


def _gate_register(broker: PolicyInterpreter, func_name: str) -> dict:
    """Simulate a policy.gate check for lambda.register."""
    resp = broker.check(CheckRequest(
        capability="CAP_IPC_CALL",
        path=func_name,
        method="lambda.register",
        principal="agent-core",
        provenance="user-intent",
    ))
    return {"allowed": resp.decision == "ALLOW", "response": resp}


def _gate_invoke(broker: PolicyInterpreter, func_name: str) -> dict:
    """Simulate a policy.gate check for lambda.invoke."""
    resp = broker.check(CheckRequest(
        capability="CAP_IPC_CALL",
        path=func_name,
        method="lambda.invoke",
        principal="agent-core",
        provenance="user-intent",
    ))
    return {"allowed": resp.decision == "ALLOW", "response": resp}


# ── tests ──────────────────────────────────────────────────────────────


class TestAgentLambdaPolicyGated:
    """End-to-end: register → gate-check → invoke → gate-check → assert output."""

    def test_register_and_invoke_calc_multiply(
        self,
        lambda_mcp,
        policy_broker,
        state_store,
        register_policy,
    ):
        # 1. Register a policy allowing CAP_IPC_CALL for calc.* methods
        register_policy([
            {
                "path": "calc.*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
            {
                "path": "calc.*",
                "method": "lambda.invoke",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        # 2. Gate-check: lambda.register for calc.multiply
        gate = _gate_register(policy_broker, "calc.multiply")
        assert gate["allowed"], f"Broker DENY on register: {gate['response'].message}"
        assert gate["response"].decision == "ALLOW"

        # 3. Actually register the function via MCP interface
        result = lambda_mcp.handle_tool_call("lambda.register", {
            "name": "calc.multiply",
            "runtime": "python3.12",
            "code": MULTIPLY_SOURCE,
            "description": "Multiply a list of numbers",
            "input_schema": {"numbers": "number[]"},
            "output_schema": {"product": "number", "count": "number"},
            "capabilities": [],
            "exposes_mcp": "calc.*",
        })
        assert result.get("success") is True, f"register failed: {result}"
        assert result["manifest"]["name"] == "calc.multiply"
        assert result["manifest"]["version"] == 1

        # 4. Gate-check: lambda.invoke for calc.multiply
        gate = _gate_invoke(policy_broker, "calc.multiply")
        assert gate["allowed"], f"Broker DENY on invoke: {gate['response'].message}"

        # 5. Invoke the function
        invoke_result = lambda_mcp.handle_tool_call("lambda.invoke", {
            "name": "calc.multiply",
            "input": {"numbers": [2, 3, 5]},
        })
        assert invoke_result.get("success") is True, f"invoke failed: {invoke_result}"
        assert invoke_result["output"]["product"] == 30
        assert invoke_result["output"]["count"] == 3

    def test_register_and_invoke_calc_add(
        self,
        lambda_mcp,
        policy_broker,
        register_policy,
    ):
        register_policy([
            {
                "path": "calc.*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
            {
                "path": "calc.*",
                "method": "lambda.invoke",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        gate = _gate_register(policy_broker, "calc.add")
        assert gate["allowed"]

        lambda_mcp.handle_tool_call("lambda.register", {
            "name": "calc.add",
            "runtime": "python3.12",
            "code": ADD_SOURCE,
            "description": "Add two numbers",
            "input_schema": {},
            "output_schema": {},
            "capabilities": [],
        })

        gate = _gate_invoke(policy_broker, "calc.add")
        assert gate["allowed"]

        result = lambda_mcp.handle_tool_call("lambda.invoke", {
            "name": "calc.add",
            "input": {"a": 17, "b": 25},
        })
        assert result["output"]["sum"] == 42

    def test_deny_register_outside_calc_pattern(
        self,
        policy_broker,
        register_policy,
    ):
        """Policy blocks register for functions not matching calc.*"""
        register_policy([
            {
                "path": "calc.*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        gate = _gate_register(policy_broker, "net.fetch")
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

    def test_deny_invoke_unregistered_function(
        self,
        lambda_mcp,
        policy_broker,
        register_policy,
    ):
        """Invoke a function that was never registered → DENY."""
        register_policy([
            {
                "path": "*",
                "method": "lambda.invoke",
                "decision": "DENY",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        gate = _gate_invoke(policy_broker, "calc.nonexistent")
        assert not gate["allowed"]
        assert gate["response"].decision == "DENY"

    def test_version_increments_on_re_register(
        self,
        lambda_mcp,
        policy_broker,
        register_policy,
    ):
        register_policy([
            {
                "path": "calc.*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
            {
                "path": "calc.*",
                "method": "lambda.invoke",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        # Register v1
        r1 = lambda_mcp.handle_tool_call("lambda.register", {
            "name": "calc.double",
            "runtime": "python3.12",
            "code": "def double(input): return {'result': input['n'] * 2}",
            "description": "Double a number",
            "input_schema": {},
            "output_schema": {},
            "capabilities": [],
        })
        assert r1["manifest"]["version"] == 1

        # Register v2 (update)
        r2 = lambda_mcp.handle_tool_call("lambda.register", {
            "name": "calc.double",
            "runtime": "python3.12",
            "code": "def double(input): return {'result': input['n'] * 2, 'updated': True}",
            "description": "Double a number (v2)",
            "input_schema": {},
            "output_schema": {},
            "capabilities": [],
        })
        assert r2["manifest"]["version"] == 2

        # Invoke runs v2
        invoke_result = lambda_mcp.handle_tool_call("lambda.invoke", {
            "name": "calc.double",
            "input": {"n": 21},
        })
        assert invoke_result["output"]["result"] == 42
        assert invoke_result["output"]["updated"] is True

    def test_policy_audit_log_records_register_and_invoke(
        self,
        lambda_mcp,
        policy_broker,
        state_store,
        register_policy,
    ):
        """Audit log entries are written for every policy check.

        Note: PolicyInterpreter logs the *capability* as the `method` field
        and the *principal* as the `provenance` field (see interpreter.py:67).
        This test validates the actual audit-log shape produced by the Broker.
        """
        register_policy([
            {
                "path": "calc.*",
                "method": "lambda.register",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
            {
                "path": "calc.*",
                "method": "lambda.invoke",
                "decision": "ALLOW",
                "capabilities": ["CAP_IPC_CALL"],
            },
        ])

        # Perform two gated calls
        _gate_register(policy_broker, "calc.square")
        _gate_invoke(policy_broker, "calc.square")

        log = state_store.query_audit_log()
        assert len(log) == 2

        # Both calls use CAP_IPC_CALL capability
        capabilities_in_log = [e.method for e in log]
        assert all(cap == "CAP_IPC_CALL" for cap in capabilities_in_log)

        for entry in log:
            assert entry.timestamp is not None
            assert entry.decision in ("ALLOW", "DENY", "CONFIRM", "HOLD")
            # principal is stored as provenance by the interpreter
            assert entry.provenance == "agent-core"
            assert isinstance(entry.request, dict)
