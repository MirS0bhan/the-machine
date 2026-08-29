"""
End-to-end test: a real LLM drives the Lambda MCP server.

The LLM (OpenAI-compatible API at http://localhost:20128/v1, model
"auto/best-coding") is given the Lambda MCP tools as a function-calling
surface. This test is the *agent loop*: it forwards the LLM's tool calls to
our running Lambda Server (http://localhost:8080) over the real HTTP/MCP
interface, feeds results back, and verifies the LLM actually registered and
executed a function whose true output is computed by the server.

Run with: uv run python test_llm_mcp.py

Environment:
    LLM_BASE   - API base URL            (default http://localhost:20128/v1)
    LLM_MODEL  - model name              (default auto/best-coding)
    LLM_API_KEY - bearer token          (default: empty / optional)
    LAMBDA_BASE - Lambda HTTP base URL   (default http://localhost:8080)
"""

import json
import os
import sys
import time
import urllib.request
import urllib.error

LLM_BASE = os.environ.get("LLM_BASE", "http://localhost:20128/v1")
LLM_MODEL = os.environ.get("LLM_MODEL", "auto/best-coding")
LLM_API_KEY = os.environ.get("LLM_API_KEY", "")
LAMBDA_BASE = os.environ.get("LAMBDA_BASE", "http://localhost:8080")

MAX_ITER = 8
MAX_RETRIES = 5
RETRY_BACKOFF = 3.0


# --------------------------------------------------------------------------
# Lambda MCP HTTP client (talks to our real running server)
# --------------------------------------------------------------------------

def lambda_mcp(tool_name: str, args: dict) -> dict:
    """Call a Lambda MCP tool on the running server."""
    url = f"{LAMBDA_BASE}/mcp/{tool_name}"
    data = json.dumps(args).encode()
    req = urllib.request.Request(
        url, data=data, headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        return json.loads(resp.read().decode())


# --------------------------------------------------------------------------
# LLM client (OpenAI-compatible chat completions with tools)
# --------------------------------------------------------------------------

def _llm_request(payload: dict) -> dict:
    """POST to /chat/completions with retry/backoff for 429s."""
    url = f"{LLM_BASE}/chat/completions"
    headers = {"Content-Type": "application/json"}
    if LLM_API_KEY:
        headers["Authorization"] = f"Bearer {LLM_API_KEY}"
    data = json.dumps(payload).encode()

    last_err = None
    for attempt in range(MAX_RETRIES):
        try:
            req = urllib.request.Request(url, data=data, headers=headers)
            with urllib.request.urlopen(req, timeout=120) as resp:
                return json.loads(resp.read().decode())
        except urllib.error.HTTPError as e:
            body = e.read().decode(errors="replace")
            last_err = f"HTTP {e.code}: {body}"
            if e.code == 429 and attempt < MAX_RETRIES - 1:
                time.sleep(RETRY_BACKOFF * (attempt + 1))
                continue
            raise
    raise RuntimeError(f"LLM request failed: {last_err}")


def llm_chat(messages: list, tools: list) -> dict:
    """One chat completion round. Returns the parsed response dict."""
    payload = {
        "model": LLM_MODEL,
        "messages": messages,
        "tools": tools,
        "stream": False,
        "temperature": 0,
    }
    return _llm_request(payload)


# --------------------------------------------------------------------------
# Tool schemas exposed to the LLM
# --------------------------------------------------------------------------

TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "lambda.register",
            "description": (
                "Create or update a Lambda function. Provide Python source "
                "code defining a function whose name matches the short "
                "function name; it receives an 'input' dict and returns a "
                "dict. Capability 'pure' means no IO."
            ),
            "parameters": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Dotted function name, e.g. calc.multiply"},
                    "runtime": {"type": "string", "description": "Runtime, e.g. python3.12"},
                    "code": {"type": "string", "description": "Python source defining the entry function"},
                    "description": {"type": "string"},
                    "input_schema": {"type": "object"},
                    "output_schema": {"type": "object"},
                    "capabilities": {"type": "string", "description": "Capability preset, e.g. 'pure'"},
                },
                "required": ["name", "runtime", "code", "description"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "lambda.invoke",
            "description": "Invoke a registered Lambda function with an input object.",
            "parameters": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "input": {"type": "object"},
                },
                "required": ["name", "input"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "lambda.search",
            "description": "Search registered functions by keyword.",
            "parameters": {
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "required": ["query"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "lambda.describe",
            "description": "Get the full manifest for a function.",
            "parameters": {
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
            },
        },
    },
]


SYSTEM_PROMPT = (
    "You are an agent that builds and runs computations using a Lambda "
    "Execution Server accessible via MCP tools. To solve a math task: "
    "1) register a function with the correct Python code using "
    "lambda.register, then 2) invoke it with lambda.invoke, then 3) report "
    "the result from the invocation output. Always use the tools rather than "
    "computing by hand."
)


# --------------------------------------------------------------------------
# Agent loop
# --------------------------------------------------------------------------

def check(label: str, ok: bool, detail="") -> bool:
    status = "PASS" if ok else "FAIL"
    print(f"  [{status}] {label}" + (f" - {detail}" if detail else ""))
    return ok


def run_agent(user_prompt: str) -> dict:
    """
    Run the agent loop. Returns a summary dict with the final answer and
    captured tool calls.
    """
    messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": user_prompt},
    ]

    register_calls = []
    invoke_results = []

    for i in range(MAX_ITER):
        resp = llm_chat(messages, TOOLS)
        choice = resp["choices"][0]
        msg = choice["message"]

        tool_calls = msg.get("tool_calls")
        if not tool_calls:
            # Final natural-language answer.
            return {
                "answer": msg.get("content", ""),
                "register_calls": register_calls,
                "invoke_results": invoke_results,
                "iterations": i + 1,
            }

        # Append the assistant message (with tool_calls) to history.
        messages.append({
            "role": "assistant",
            "content": msg.get("content", ""),
            "tool_calls": tool_calls,
        })

        for tc in tool_calls:
            fn = tc["function"]
            name = fn["name"]
            try:
                args = json.loads(fn.get("arguments") or "{}")
            except json.JSONDecodeError:
                args = {}

            print(f"  -> LLM called {name}({json.dumps(args)[:120]})")
            result = lambda_mcp(name, args)

            if name == "lambda.register":
                register_calls.append(args)
            if name == "lambda.invoke":
                invoke_results.append(result)

            messages.append({
                "role": "tool",
                "tool_call_id": tc["id"],
                "name": name,
                "content": json.dumps(result),
            })

    return {
        "answer": "(max iterations reached)",
        "register_calls": register_calls,
        "invoke_results": invoke_results,
        "iterations": MAX_ITER,
    }


def main() -> int:
    print("=" * 60)
    print("Lambda Server + Real LLM (MCP agent) Test")
    print("=" * 60)
    print(f"LLM:  {LLM_BASE}  model={LLM_MODEL}")
    print(f"Lambda: {LAMBDA_BASE}")
    print()

    results = []

    prompt = (
        "Register a Lambda function named 'calc.multiply' with runtime "
        "python3.12 and capability 'pure'. Its code defines a function "
        "multiply(input) that takes {'numbers': [...]} and returns "
        "{'product': <product of all numbers>, 'count': <how many>}. "
        "Then invoke calc.multiply with input {'numbers': [2, 3, 5, 7]} "
        "and tell me the product."
    )

    print("Running agent loop...\n")
    try:
        summary = run_agent(prompt)
    except urllib.error.HTTPError as e:
        body = e.read().decode(errors="replace")[:600]
        print(f"\n[FAIL] LLM backend returned HTTP {e.code}: {body}")
        print("       (This is an upstream quota/policy error from the LLM "
              "proxy, not a Lambda Server issue.)")
        return 1
    except Exception as e:
        print(f"\n[FAIL] Agent loop crashed: {e}")
        return 1

    print(f"\nFinal answer: {summary['answer']!r}\n")

    # 1. LLM must have registered a function.
    registered = any(
        c.get("name") == "calc.multiply" for c in summary["register_calls"]
    )
    results.append(check("LLM registered calc.multiply via MCP", registered,
                         f"{len(summary['register_calls'])} register call(s)"))

    # 2. LLM must have invoked it, and the SERVER-computed product is 210.
    products = []
    for r in summary["invoke_results"]:
        out = r.get("output", {})
        if isinstance(out, dict) and "product" in out:
            products.append(out["product"])
    correct = 210 in products
    results.append(check("Server executed function, product == 210", correct,
                         f"invoke outputs: {products}"))

    # 3. LLM's final answer should mention the result.
    mentioned = "210" in summary["answer"]
    results.append(check("LLM reported 210 in final answer", mentioned))

    print("\n" + "=" * 60)
    passed = sum(1 for r in results if r)
    print(f"SUMMARY: {passed}/{len(results)} passed")
    print("=" * 60)
    return 0 if passed == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
