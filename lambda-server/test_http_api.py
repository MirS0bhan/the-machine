"""
Test script for the Lambda Execution Server HTTP API.
Run with: uv run python test_http_api.py
"""
import json
import sys
import time
import urllib.request
import urllib.error

BASE_URL = "http://localhost:8080"


def api_get(path: str):
    """GET request returning parsed JSON."""
    req = urllib.request.Request(f"{BASE_URL}{path}")
    with urllib.request.urlopen(req, timeout=10) as resp:
        return json.loads(resp.read().decode())


def api_post(path: str, payload: dict):
    """POST JSON request returning parsed JSON."""
    data = json.dumps(payload).encode()
    req = urllib.request.Request(
        f"{BASE_URL}{path}", data=data,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode())


def check(label: str, ok: bool, detail=""):
    status = "PASS" if ok else "FAIL"
    print(f"  [{status}] {label}" + (f" - {detail}" if detail else ""))
    return ok


def main():
    print("=" * 60)
    print("Lambda Server HTTP API Test")
    print("=" * 60)
    results = []

    # 1. Health check
    print("\n1. Health check")
    try:
        health = api_get("/health")
        results.append(check("GET /health", health.get("status") in ("ok", "healthy"), str(health)))
    except Exception as e:
        results.append(check("GET /health", False, str(e)))

    # 2. List tools
    print("\n2. List tools")
    try:
        tools = api_get("/tools")
        names = tools.get("tools", [])
        results.append(check("GET /tools", len(names) > 0, f"{len(names)} tools"))
    except Exception as e:
        results.append(check("GET /tools", False, str(e)))

    # 3. Register a function
    print("\n3. Register calc.multiply")
    try:
        reg = api_post("/mcp/lambda.register", {
            "name": "calc.multiply",
            "runtime": "python3.12",
            "code": (
                "def multiply(input):\n"
                "    numbers = input['numbers']\n"
                "    product = 1\n"
                "    for n in numbers:\n"
                "        product *= n\n"
                "    return {'product': product, 'count': len(numbers)}\n"
            ),
            "description": "Multiplies a list of numbers",
            "input_schema": {"numbers": "number[]"},
            "output_schema": {"product": "number", "count": "number"},
            "capabilities": "pure",
        })
        results.append(check("register calc.multiply", reg.get("success") is True, str(reg)))
    except Exception as e:
        results.append(check("register calc.multiply", False, str(e)))

    # 4. Search
    print("\n4. Search 'multiply'")
    try:
        search = api_post("/mcp/lambda.search", {"query": "multiply"})
        found = any(f.get("name") == "calc.multiply" for f in search.get("results", []))
        results.append(check("search finds calc.multiply", found, str(search)))
    except Exception as e:
        results.append(check("search finds calc.multiply", False, str(e)))

    # 5. Describe
    print("\n5. Describe calc.multiply")
    try:
        desc = api_post("/mcp/lambda.describe", {"name": "calc.multiply"})
        results.append(check("describe returns manifest", bool(desc.get("manifest")), str(desc)))
    except Exception as e:
        results.append(check("describe returns manifest", False, str(e)))

    # 6. Invoke
    print("\n6. Invoke calc.multiply with [2, 3, 5, 7]")
    try:
        invoke = api_post("/mcp/lambda.invoke", {
            "name": "calc.multiply",
            "input": {"numbers": [2, 3, 5, 7]},
        })
        output = invoke.get("output", {})
        expected = 2 * 3 * 5 * 7  # 210
        ok = invoke.get("success") and output.get("product") == expected
        results.append(check(f"result = {expected}", ok, str(output)))
    except Exception as e:
        results.append(check("result = 210", False, str(e)))

    # 7. List functions
    print("\n7. List functions")
    try:
        lst = api_post("/mcp/lambda.list_functions", {})
        count = len(lst.get("functions", []))
        results.append(check("list_functions", count >= 1, f"{count} function(s)"))
    except Exception as e:
        results.append(check("list_functions", False, str(e)))

    # 8. Stats
    print("\n8. Stats")
    try:
        stats = api_post("/mcp/lambda.get_stats", {})
        results.append(check("get_stats", bool(stats.get("total_processes", 0) >= 0), str(stats)))
    except Exception as e:
        results.append(check("get_stats", False, str(e)))

    # 9. Invalid request
    print("\n9. Invalid function (should fail gracefully)")
    try:
        bad = api_post("/mcp/lambda.invoke", {"name": "nonexistent.func", "input": {}})
        ok = bad.get("success") is False
        results.append(check("unknown function rejected", ok, str(bad)))
    except Exception as e:
        results.append(check("unknown function rejected", False, str(e)))

    # Summary
    print("\n" + "=" * 60)
    passed = sum(1 for r in results if r)
    print(f"SUMMARY: {passed}/{len(results)} passed")
    print("=" * 60)
    return 0 if passed == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
