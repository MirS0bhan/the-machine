#!/usr/bin/env python3
"""
Example usage of the Lambda Execution Server.

This demonstrates the workflow from spec §8:
"Calculate something" → search → miss → register → invoke
"""

import sys
import os

# Add lambda-server to path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from server import create_server


def main():
    # Create server instance
    server = create_server()
    
    print("Lambda Execution Server Example")
    print("=" * 50)
    
    # Step 1: Search for a calculator function
    print("\n1. Searching for 'calculate'...")
    result = server.handle_mcp_tool("lambda.search", {"query": "calculate"})
    print(f"   Found {result['count']} results")
    
    # Step 2: Register a calculator function
    print("\n2. Registering calc.eval function...")
    result = server.handle_mcp_tool("lambda.register", {
        "name": "calc.eval",
        "runtime": "python3.12",
        "code": """
def calc_eval(input):
    \"\"\"Evaluate a mathematical expression.\"\"\"
    expression = input.get("expression", "0")
    # In production, this would use a safe evaluator
    result = eval(expression)
    return {"result": result}
""",
        "description": "Evaluates a mathematical expression and returns the result",
        "input_schema": {"expression": "string"},
        "output_schema": {"result": "number"},
        "capabilities": "pure",
        "exposes_mcp": "calc.*",
    })
    print(f"   Registered: {result['success']}")
    
    # Step 3: Search again (should find it)
    print("\n3. Searching for 'expression'...")
    result = server.handle_mcp_tool("lambda.search", {"query": "expression"})
    print(f"   Found {result['count']} results")
    if result['count'] > 0:
        print(f"   First result: {result['results'][0]['name']}")
    
    # Step 4: Invoke the function
    print("\n4. Invoking calc.eval with '47 * 12.5'...")
    result = server.handle_mcp_tool("lambda.invoke", {
        "name": "calc.eval",
        "input": {"expression": "47 * 12.5"},
    })
    print(f"   Success: {result['success']}")
    if result['success']:
        print(f"   Output: {result['output']}")
    
    # Step 5: Describe the function
    print("\n5. Describing calc.eval...")
    result = server.handle_mcp_tool("lambda.describe", {"name": "calc.eval"})
    if 'manifest' in result:
        manifest = result['manifest']
        print(f"   Name: {manifest['name']}")
        print(f"   Version: {manifest['version']}")
        print(f"   Runtime: {manifest['runtime']}")
        print(f"   MCP Exposure: {manifest['exposes_mcp']}")
        print(f"   History: {len(result['history'])} version(s)")
    
    # Step 6: List all functions
    print("\n6. Listing all functions...")
    result = server.handle_mcp_tool("lambda.list_functions", {})
    print(f"   Total functions: {result['count']}")
    
    # Step 7: Get server stats
    print("\n7. Getting server stats...")
    result = server.handle_mcp_tool("lambda.get_stats", {})
    print(f"   Functions registered: {result.get('functions_registered', 0)}")
    print(f"   Active leases: {result.get('leases_active', 0)}")
    
    print("\n" + "=" * 50)
    print("Example complete!")


if __name__ == "__main__":
    main()
