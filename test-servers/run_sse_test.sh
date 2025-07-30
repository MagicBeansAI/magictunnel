#!/bin/bash

# Complete SSE MCP Testing Workflow Script
# This script sets up and tests MagicTunnel's SSE client against a test SSE server

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Please run this script from the MagicTunnel root directory"
    exit 1
fi

print_status "Starting SSE MCP Testing Workflow"
echo "=================================================="

# Step 1: Check dependencies
print_status "Step 1: Checking dependencies..."

if ! command -v python3 &> /dev/null; then
    print_error "python3 is required but not installed"
    exit 1
fi

if ! python3 -c "import fastapi, uvicorn" 2>/dev/null; then
    print_warning "FastAPI/Uvicorn not found. Installing..."
    pip3 install fastapi uvicorn
fi

print_success "Dependencies checked"

# Step 2: Build MagicTunnel
print_status "Step 2: Building MagicTunnel..."
cargo build --release
print_success "MagicTunnel built successfully"

# Step 3: Start SSE Test Server
print_status "Step 3: Starting SSE test server..."

# Kill any existing server on port 8000
if lsof -ti:8000 >/dev/null 2>&1; then
    print_warning "Killing existing process on port 8000"
    kill -9 $(lsof -ti:8000) || true
    sleep 2
fi

# Start the SSE server in background
python3 test-servers/simple_sse_mcp_server.py --port 8000 &
SSE_SERVER_PID=$!

# Wait for server to start
print_status "Waiting for SSE server to start..."
sleep 3

# Step 4: Test SSE Server
print_status "Step 4: Testing SSE server..."
if python3 test-servers/test_sse_server.py; then
    print_success "SSE server is working correctly"
else
    print_error "SSE server test failed"
    kill $SSE_SERVER_PID 2>/dev/null || true
    exit 1
fi

# Step 5: Start MagicTunnel
print_status "Step 5: Starting MagicTunnel with SSE configuration..."

# Kill any existing MagicTunnel process
if lsof -ti:8080 >/dev/null 2>&1; then
    print_warning "Killing existing process on port 8080"
    kill -9 $(lsof -ti:8080) || true
    sleep 2
fi

# Start MagicTunnel with debug logging
RUST_LOG=magictunnel::mcp::clients::sse_client=debug,magictunnel::mcp::network_service_manager=debug \
./target/release/magictunnel --config test-servers/sse-test-config.yaml &
MAGICTUNNEL_PID=$!

# Wait for MagicTunnel to start
print_status "Waiting for MagicTunnel to start..."
sleep 5

# Step 6: Test MagicTunnel SSE Integration
print_status "Step 6: Testing MagicTunnel SSE integration..."

# Test 1: Check MagicTunnel health
print_status "Testing MagicTunnel health endpoint..."
if curl -s http://localhost:8080/health >/dev/null; then
    print_success "MagicTunnel is responding"
else
    print_error "MagicTunnel is not responding"
    kill $SSE_SERVER_PID $MAGICTUNNEL_PID 2>/dev/null || true
    exit 1
fi

# Test 2: List all tools (should include SSE server tools)
print_status "Testing tool discovery..."
TOOLS_RESPONSE=$(curl -s -X POST http://localhost:8080/v1/mcp/list_tools \
    -H "Content-Type: application/json" \
    -d '{}')

if echo "$TOOLS_RESPONSE" | grep -q "echo"; then
    print_success "SSE server tools discovered successfully"
    echo "Available tools:"
    echo "$TOOLS_RESPONSE" | python3 -m json.tool | grep -A 1 -B 1 '"name"'
else
    print_warning "SSE server tools not found in tool list"
    echo "Tools response: $TOOLS_RESPONSE"
fi

# Test 3: Use smart discovery to find and execute SSE tools
print_status "Testing smart discovery with SSE tools..."

# Test echo tool
ECHO_RESPONSE=$(curl -s -X POST http://localhost:8080/v1/mcp/call \
    -H "Content-Type: application/json" \
    -d '{
        "name": "smart_tool_discovery",
        "arguments": {
            "request": "echo hello from SSE server"
        }
    }')

if echo "$ECHO_RESPONSE" | grep -q "Echo: hello from SSE server"; then
    print_success "Smart discovery with SSE echo tool works!"
else
    print_warning "Smart discovery with SSE tools may have issues"
    echo "Echo response: $ECHO_RESPONSE"
fi

# Test time tool
TIME_RESPONSE=$(curl -s -X POST http://localhost:8080/v1/mcp/call \
    -H "Content-Type: application/json" \
    -d '{
        "name": "smart_tool_discovery",
        "arguments": {
            "request": "get current server time"
        }
    }')

if echo "$TIME_RESPONSE" | grep -q "Current server time"; then
    print_success "Smart discovery with SSE time tool works!"
else
    print_warning "Smart discovery with time tool may have issues"
    echo "Time response: $TIME_RESPONSE"
fi

# Step 7: Test concurrent requests (SSE queuing)
print_status "Step 7: Testing concurrent requests (SSE request queuing)..."

# Launch multiple concurrent requests
for i in {1..5}; do
    curl -s -X POST http://localhost:8080/v1/mcp/call \
        -H "Content-Type: application/json" \
        -d "{\"name\": \"smart_tool_discovery\", \"arguments\": {\"request\": \"echo test message $i\"}}" &
done

# Wait for all requests to complete
wait

print_success "Concurrent request test completed"

# Step 8: Check capability files
print_status "Step 8: Checking generated capability files..."

if ls capabilities/network-sse-*.yaml >/dev/null 2>&1; then
    print_success "SSE capability files generated:"
    ls -la capabilities/network-sse-*.yaml
else
    print_warning "No SSE capability files found in ./capabilities/"
fi

# Step 9: Summary and cleanup
print_status "Step 9: Test summary and cleanup..."

# Give user time to see results
print_status "Tests completed! Servers will continue running for manual testing."
print_status "You can now:"
echo "  • Open http://localhost:8080 to see MagicTunnel"
echo "  • Open http://localhost:8000 to see SSE test server"
echo "  • Check logs for SSE connection details"
echo "  • Run additional manual tests"
echo ""
print_status "Press Enter to stop all servers..."
read

# Cleanup
print_status "Stopping servers..."
kill $SSE_SERVER_PID $MAGICTUNNEL_PID 2>/dev/null || true

print_success "SSE MCP testing workflow completed!"

echo ""
echo "=================================================="
echo "🎯 Next Steps:"
echo "• Check the logs above for any connection issues"
echo "• Try the Web Dashboard at http://localhost:5173 (if using supervisor)"
echo "• Test with other SSE MCP servers from the internet"
echo "• Review generated capability files in ./capabilities/"
echo "=================================================="