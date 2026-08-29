"""Tests for the Lambda Execution Server."""

import sys
import os

# Add lambda-server to path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from server import LambdaServer
from models import Capability, CapabilityGrant


def test_registry():
    """Test function registry operations."""
    server = LambdaServer()
    
    # Register a function
    result = server.handle_mcp_tool("lambda.register", {
        "name": "calc.add",
        "runtime": "python3.12",
        "code": "def add(input): return {'sum': sum(input['values'])}",
        "description": "Adds two or more numeric values",
        "input_schema": {"values": "number[]"},
        "output_schema": {"sum": "number"},
        "capabilities": "pure",
    })
    assert result["success"], f"Registration failed: {result}"
    print("✓ Function registered")
    
    # Search for function
    result = server.handle_mcp_tool("lambda.search", {"query": "add"})
    assert result["count"] > 0, "Search should find the function"
    print("✓ Search found function")
    
    # Describe function
    result = server.handle_mcp_tool("lambda.describe", {"name": "calc.add"})
    assert "manifest" in result, "Describe should return manifest"
    assert result["manifest"]["name"] == "calc.add"
    print("✓ Describe returned correct manifest")
    
    # Invoke function
    result = server.handle_mcp_tool("lambda.invoke", {
        "name": "calc.add",
        "input": {"values": [1, 2, 3]},
    })
    assert result["success"], f"Invoke failed: {result}"
    print("✓ Function invoked")
    
    # List functions
    result = server.handle_mcp_tool("lambda.list_functions", {})
    assert result["count"] > 0, "Should list functions"
    print("✓ Listed functions")
    
    print("\n✓ Registry tests passed\n")


def test_capabilities():
    """Test capability enforcement."""
    from enforcer import CapabilityEnforcer
    from models import FunctionManifest, FunctionVersion
    
    enforcer = CapabilityEnforcer()
    
    # Test valid manifest
    manifest = FunctionManifest(
        name="test",
        version=1,
        runtime="python3.12",
        description="Test function",
        input_schema={},
        output_schema={},
        capabilities={
            CapabilityGrant(
                capability=Capability.IPC_CALL,
                targets=("other_func",),
            )
        },
        source_code="pass",
    )
    
    result = enforcer.validate_manifest(manifest)
    assert result.allowed, f"Validation failed: {result.reason}"
    print("✓ Valid manifest accepted")
    
    # Test invalid manifest (NET_OUT without domains)
    manifest2 = FunctionManifest(
        name="test2",
        version=1,
        runtime="python3.12",
        description="Test function 2",
        input_schema={},
        output_schema={},
        capabilities={
            CapabilityGrant(
                capability=Capability.NET_OUT,
            )
        },
        source_code="pass",
    )
    
    result2 = enforcer.validate_manifest(manifest2)
    assert not result2.allowed, "Should reject NET_OUT without domains"
    print("✓ Invalid manifest rejected")
    
    # Test IPC call check
    manifest3 = FunctionManifest(
        name="caller",
        version=1,
        runtime="python3.12",
        description="Caller",
        input_schema={},
        output_schema={},
        capabilities={
            CapabilityGrant(
                capability=Capability.IPC_CALL,
                targets=("target",),
            )
        },
        source_code="pass",
    )
    
    result3 = enforcer.check_ipc_call("caller", "target", manifest3)
    assert result3.allowed, f"IPC call should be allowed: {result3.reason}"
    print("✓ IPC call allowed")
    
    result4 = enforcer.check_ipc_call("caller", "unauthorized", manifest3)
    assert not result4.allowed, "IPC call to unauthorized target should fail"
    print("✓ Unauthorized IPC call rejected")
    
    # Test preset expansion
    grants = enforcer.expand_preset("pure")
    assert len(grants) == 0, "Pure preset should have no capabilities"
    print("✓ Pure preset expanded")
    
    grants = enforcer.expand_preset("reader")
    assert any(g.capability == Capability.STATE_READ for g in grants), "Reader should have STATE_READ"
    print("✓ Reader preset expanded")
    
    print("\n✓ Capability tests passed\n")


def test_mcp_tools():
    """Test MCP tool interface."""
    server = LambdaServer()
    
    # Get available tools
    tools = server.get_tools()
    assert len(tools) > 0, "Should have tools"
    print(f"✓ Found {len(tools)} MCP tools")
    
    # Test with invalid tool
    result = server.handle_mcp_tool("invalid.tool", {})
    assert "error" in result, "Invalid tool should return error"
    print("✓ Invalid tool handled")
    
    # Test describe with non-existent function
    result = server.handle_mcp_tool("lambda.describe", {"name": "nonexistent"})
    assert "error" in result, "Should return error for missing function"
    print("✓ Missing function handled")
    
    print("\n✓ MCP tool tests passed\n")


def test_supervisor():
    """Test process supervisor."""
    from supervisor import ProcessSupervisor
    from models import FunctionManifest
    
    supervisor = ProcessSupervisor()
    
    # Spawn a process
    manifest = FunctionManifest(
        name="test_func",
        version=1,
        runtime="python3.12",
        description="Test",
        input_schema={},
        output_schema={},
        capabilities=set(),
        source_code="pass",
    )
    
    handle = supervisor.spawn(manifest)
    assert handle.status == "running", f"Process should be running: {handle.status}"
    print("✓ Process spawned")
    
    # Get process
    got = supervisor.get_process(handle.process_id)
    assert got is not None, "Should get process"
    print("✓ Process retrieved")
    
    # List processes
    processes = supervisor.list_processes()
    assert len(processes) > 0, "Should list processes"
    print("✓ Listed processes")
    
    # Get stats
    stats = supervisor.get_stats()
    assert "total_processes" in stats, "Stats should have total_processes"
    print("✓ Got stats")
    
    print("\n✓ Supervisor tests passed\n")


def test_router():
    """Test IPC router."""
    from router import IPCRouter
    
    router = IPCRouter()
    
    # Get active leases (should be empty)
    leases = router.get_active_leases()
    assert len(leases) == 0, "Should start with no leases"
    print("✓ Initial lease list empty")
    
    # Get call log (should be empty)
    logs = router.get_call_log()
    assert len(logs) == 0, "Should start with no logs"
    print("✓ Initial call log empty")
    
    print("\n✓ Router tests passed\n")


def test_sdk():
    """Test Python SDK."""
    from sdk import capabilities, call, state, LambdaFunction
    
    # Test capabilities decorator
    @capabilities(ipc_call=["target"])
    def my_func(input):
        return call("target", input)
    
    caps = my_func._lambda_capabilities
    assert caps["ipc_call"] == ["target"], "Should have IPC call capability"
    print("✓ Capabilities decorator works")
    
    # Test LambdaFunction
    func = LambdaFunction(
        name="test",
        func=my_func,
        description="Test function",
    )
    assert func.name == "test", "Function name should be set"
    assert func.capabilities["ipc_call"] == ["target"], "Should have capabilities"
    print("✓ LambdaFunction created")
    
    print("\n✓ SDK tests passed\n")


def test_end_to_end():
    """End-to-end test of the workflow from the spec."""
    server = LambdaServer()
    
    print("Testing end-to-end workflow from spec §8:")
    print("  'Calculate something' → search → miss → register → invoke\n")
    
    # Step 1: Search (should miss)
    result = server.handle_mcp_tool("lambda.search", {"query": "calculate"})
    print(f"1. Search: {result['count']} results")
    assert result["count"] == 0, "Should be empty initially"
    
    # Step 2: Register calculator
    result = server.handle_mcp_tool("lambda.register", {
        "name": "calc.eval",
        "runtime": "python3.12",
        "code": "def calc_eval(input): return {'result': eval(input['expression'])}",
        "description": "Evaluates a mathematical expression",
        "input_schema": {"expression": "string"},
        "output_schema": {"result": "number"},
        "capabilities": "pure",
        "exposes_mcp": "calc.*",
    })
    assert result["success"], f"Registration failed: {result}"
    print("2. Registered calc.eval")
    
    # Step 3: Search again (should hit)
    result = server.handle_mcp_tool("lambda.search", {"query": "expression"})
    print(f"3. Search: {result['count']} results")
    assert result["count"] > 0, "Should find the function"
    
    # Step 4: Invoke
    result = server.handle_mcp_tool("lambda.invoke", {
        "name": "calc.eval",
        "input": {"expression": "47 * 12.5"},
    })
    assert result["success"], f"Invoke failed: {result}"
    print(f"4. Invoked: {result['output']}")
    
    # Step 5: Check MCP exposure
    result = server.handle_mcp_tool("lambda.describe", {"name": "calc.eval"})
    assert result["manifest"]["exposes_mcp"] == "calc.*", "Should expose MCP"
    print("5. MCP exposure verified")
    
    # Step 6: Version history
    assert len(result["history"]) == 1, "Should have 1 version"
    print("6. Version history correct")
    
    print("\n✓ End-to-end test passed\n")


if __name__ == "__main__":
    print("=" * 60)
    print("Lambda Execution Server - Test Suite")
    print("=" * 60 + "\n")
    
    test_registry()
    test_capabilities()
    test_mcp_tools()
    test_supervisor()
    test_router()
    test_sdk()
    test_end_to_end()
    
    print("=" * 60)
    print("All tests passed!")
    print("=" * 60)
