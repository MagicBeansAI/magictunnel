#!/usr/bin/env python3
"""
Test script to verify the simple SSE MCP server is working correctly

Usage:
    python3 test-servers/test_sse_server.py
"""

import json
import requests
import asyncio
from datetime import datetime

def test_health_check():
    """Test the health check endpoint"""
    print("🔍 Testing health check endpoint...")
    try:
        response = requests.get("http://127.0.0.1:8000/")
        response.raise_for_status()
        data = response.json()
        print(f"✅ Health check passed: {data['status']}")
        print(f"📋 Available tools: {', '.join(data['tools'])}")
        return True
    except Exception as e:
        print(f"❌ Health check failed: {e}")
        return False

def test_mcp_initialize():
    """Test MCP initialize request"""
    print("🔍 Testing MCP initialize...")
    try:
        request = {
            "jsonrpc": "2.0",
            "id": "test-init",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }
        
        response = requests.post("http://127.0.0.1:8000/mcp", json=request)
        response.raise_for_status()
        data = response.json()
        
        if "result" in data:
            print("✅ Initialize request successful")
            print(f"📋 Protocol Version: {data['result']['protocolVersion']}")
            print(f"📋 Server: {data['result']['serverInfo']['name']} v{data['result']['serverInfo']['version']}")
            return True
        else:
            print(f"❌ Initialize request failed: {data}")
            return False
            
    except Exception as e:
        print(f"❌ Initialize request failed: {e}")
        return False

def test_list_tools():
    """Test listing tools"""
    print("🔍 Testing tools/list...")
    try:
        request = {
            "jsonrpc": "2.0",
            "id": "test-list",
            "method": "tools/list",
            "params": {}
        }
        
        response = requests.post("http://127.0.0.1:8000/mcp", json=request)
        response.raise_for_status()
        data = response.json()
        
        if "result" in data and "tools" in data["result"]:
            tools = data["result"]["tools"]
            print(f"✅ Found {len(tools)} tools:")
            for tool in tools:
                print(f"   - {tool['name']}: {tool['description']}")
            return True
        else:
            print(f"❌ List tools failed: {data}")
            return False
            
    except Exception as e:
        print(f"❌ List tools failed: {e}")
        return False

def test_call_tool(tool_name, arguments):
    """Test calling a specific tool"""
    print(f"🔍 Testing tools/call for '{tool_name}'...")
    try:
        request = {
            "jsonrpc": "2.0",
            "id": f"test-call-{tool_name}",
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        }
        
        response = requests.post("http://127.0.0.1:8000/mcp", json=request)
        response.raise_for_status()
        data = response.json()
        
        if "result" in data:
            content = data["result"]["content"][0]["text"]
            print(f"✅ Tool '{tool_name}' executed successfully:")
            print(f"   Result: {content}")
            return True
        else:
            print(f"❌ Tool '{tool_name}' failed: {data}")
            return False
            
    except Exception as e:
        print(f"❌ Tool '{tool_name}' failed: {e}")
        return False

def test_sse_connection():
    """Basic test of SSE endpoint connectivity"""
    print("🔍 Testing SSE endpoint connectivity...")
    try:
        response = requests.get("http://127.0.0.1:8000/sse", stream=True, timeout=5)
        response.raise_for_status()
        print("✅ SSE endpoint is accessible")
        return True
    except requests.exceptions.Timeout:
        print("✅ SSE endpoint is accessible (timeout expected for streaming)")
        return True
    except Exception as e:
        print(f"❌ SSE endpoint failed: {e}")
        return False

def main():
    """Run all tests"""
    print("🚀 Starting SSE MCP Server Tests")
    print("=" * 50)
    
    tests_passed = 0
    total_tests = 6
    
    # Test 1: Health Check
    if test_health_check():
        tests_passed += 1
    print()
    
    # Test 2: MCP Initialize
    if test_mcp_initialize():
        tests_passed += 1
    print()
    
    # Test 3: List Tools
    if test_list_tools():
        tests_passed += 1
    print()
    
    # Test 4: Call Echo Tool
    if test_call_tool("echo", {"message": "Hello, SSE MCP Server!"}):
        tests_passed += 1
    print()
    
    # Test 5: Call Current Time Tool
    if test_call_tool("current_time", {}):
        tests_passed += 1
    print()
    
    # Test 6: SSE Connection
    if test_sse_connection():
        tests_passed += 1
    print()
    
    # Summary
    print("=" * 50)
    print(f"📊 Test Results: {tests_passed}/{total_tests} tests passed")
    
    if tests_passed == total_tests:
        print("✅ All tests passed! SSE MCP Server is working correctly.")
        print("🎯 Ready to test with MagicTunnel!")
    else:
        print("❌ Some tests failed. Check the server setup.")
        print("💡 Make sure the SSE server is running:")
        print("   python3 test-servers/simple_sse_mcp_server.py")
    
    return tests_passed == total_tests

if __name__ == "__main__":
    success = main()
    exit(0 if success else 1)