"""
Cross-component integration tests: Agent Core → Lambda Server (`lambda.search`).

Workflow:
    1. Agent Core registers functions via `lambda.register` (policy-gated).
    2. Agent Core searches the registry via `lambda.search`.
    3. Results rank matching functions by description keywords.

Assertions:
    - Registered functions appear in search results for matching queries.
    - Unrelated queries return empty or smaller result sets.
    - Missing query returns an error from the MCP interface.
"""

from policy_broker.models import CheckRequest

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


def _gate_register(broker, func_name: str) -> dict:
    resp = broker.check(CheckRequest(
        capability="CAP_IPC_CALL",
        path=func_name,
        method="lambda.register",
        principal="agent-core",
        provenance="user-intent",
    ))
    return {"allowed": resp.decision == "ALLOW", "response": resp}


def _register_calc_functions(lambda_mcp, policy_broker, register_policy):
    register_policy([
        {
            "path": "calc.*",
            "method": "lambda.register",
            "decision": "ALLOW",
            "capabilities": ["CAP_IPC_CALL"],
        },
    ])

    for name, code, description in (
        ("calc.multiply", MULTIPLY_SOURCE, "Multiply a list of numbers"),
        ("calc.add", ADD_SOURCE, "Add two numbers"),
    ):
        gate = _gate_register(policy_broker, name)
        assert gate["allowed"], f"Broker DENY on register: {gate['response'].message}"
        result = lambda_mcp.handle_tool_call("lambda.register", {
            "name": name,
            "runtime": "python3.12",
            "code": code,
            "description": description,
            "input_schema": {},
            "output_schema": {},
            "capabilities": [],
        })
        assert result.get("success") is True, f"register failed: {result}"


class TestLambdaSearchIntegration:
    """End-to-end: register → search → assert ranked registry metadata."""

    def test_search_finds_registered_function_by_keyword(
        self,
        lambda_mcp,
        policy_broker,
        register_policy,
    ):
        _register_calc_functions(lambda_mcp, policy_broker, register_policy)

        result = lambda_mcp.handle_tool_call("lambda.search", {"query": "multiply"})
        assert "error" not in result, result
        assert result["count"] >= 1
        names = {item["name"] for item in result["results"]}
        assert "calc.multiply" in names

    def test_search_prefers_matching_description_over_unrelated(
        self,
        lambda_mcp,
        policy_broker,
        register_policy,
    ):
        _register_calc_functions(lambda_mcp, policy_broker, register_policy)

        result = lambda_mcp.handle_tool_call("lambda.search", {"query": "add two"})
        assert result["count"] >= 1
        names = [item["name"] for item in result["results"]]
        assert "calc.add" in names

    def test_search_returns_empty_for_unknown_query(
        self,
        lambda_mcp,
        policy_broker,
        register_policy,
    ):
        _register_calc_functions(lambda_mcp, policy_broker, register_policy)

        result = lambda_mcp.handle_tool_call("lambda.search", {"query": "nonexistent-xyz"})
        assert result["count"] == 0
        assert result["results"] == []

    def test_search_requires_query(
        self,
        lambda_mcp,
    ):
        result = lambda_mcp.handle_tool_call("lambda.search", {})
        assert result.get("error") == "query is required"
