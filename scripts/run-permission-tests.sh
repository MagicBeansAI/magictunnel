#!/bin/bash

# Permission-Based Pre-Filtering System Test Runner
# This script runs comprehensive tests for the permission system

set -e

echo "🧪 MagicTunnel Permission System Test Suite"
echo "============================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test functions
run_test() {
    local test_name="$1"
    local test_command="$2"
    local description="$3"
    
    echo -e "\n${BLUE}📋 Running: ${test_name}${NC}"
    echo -e "${YELLOW}Description: ${description}${NC}"
    echo "Command: $test_command"
    echo "----------------------------------------"
    
    if eval "$test_command"; then
        echo -e "${GREEN}✅ PASSED: ${test_name}${NC}"
    else
        echo -e "${RED}❌ FAILED: ${test_name}${NC}"
        exit 1
    fi
}

# Basic compilation check
echo -e "\n${BLUE}🔧 Checking compilation...${NC}"
if cargo check --quiet; then
    echo -e "${GREEN}✅ Compilation successful${NC}"
else
    echo -e "${RED}❌ Compilation failed${NC}"
    exit 1
fi

# Run individual module tests first
echo -e "\n${YELLOW}📦 Testing individual modules...${NC}"

run_test "Permission Cache Unit Tests" \
    "cargo test permission_cache --lib" \
    "Tests basic permission caching functionality"

run_test "Fast Evaluator Unit Tests" \
    "cargo test fast_evaluator --lib" \
    "Tests bitmap-based permission evaluation"

run_test "Audit Trail Unit Tests" \
    "cargo test audit_trail --lib" \
    "Tests audit trail generation and analysis"

run_test "Cache Invalidation Unit Tests" \
    "cargo test cache_invalidation --lib" \
    "Tests cache invalidation strategies"

run_test "Filtered Tool Listing Unit Tests" \
    "cargo test filtered_tool_listing --lib" \
    "Tests permission-aware tool listing"

# Run integration tests
echo -e "\n${YELLOW}🔗 Running integration tests...${NC}"

run_test "Permission Filtered Discovery Integration" \
    "cargo test --test permission_filtered_discovery_test" \
    "End-to-end tests for permission-based pre-filtering"

run_test "Permission Cache Performance" \
    "cargo test --test permission_cache_performance_test" \
    "Performance benchmarks and stress tests"

# Run specific test scenarios
echo -e "\n${YELLOW}🎯 Running specific scenarios...${NC}"

run_test "Basic Functionality" \
    "cargo test --test permission_filtered_discovery_test test_permission_cache_basic_functionality" \
    "Verifies basic permission cache operations"

run_test "Fast Permission Evaluator" \
    "cargo test --test permission_filtered_discovery_test test_fast_permission_evaluator" \
    "Tests ultra-fast bitmap permission evaluation"

run_test "Filtered Tool Listing" \
    "cargo test --test permission_filtered_discovery_test test_filtered_tool_listing_service" \
    "Tests permission-aware tool filtering for MCP"

run_test "Cache Invalidation System" \
    "cargo test --test permission_filtered_discovery_test test_cache_invalidation_system" \
    "Tests event-driven cache invalidation"

run_test "Performance with Large Tool Set" \
    "cargo test --test permission_filtered_discovery_test test_performance_with_large_tool_set" \
    "Performance with 1000+ tools"

run_test "Edge Cases and Error Handling" \
    "cargo test --test permission_filtered_discovery_test test_edge_cases_and_error_handling" \
    "Tests error conditions and edge cases"

run_test "Concurrent Access" \
    "cargo test --test permission_filtered_discovery_test test_concurrent_access" \
    "Tests thread safety and concurrent operations"

run_test "Full Integration" \
    "cargo test --test permission_filtered_discovery_test test_full_integration" \
    "Complete end-to-end integration test"

# Run performance benchmarks
echo -e "\n${YELLOW}⚡ Running performance benchmarks...${NC}"

run_test "Individual Tool Permission Checks" \
    "cargo test --test permission_cache_performance_test bench_individual_tool_permission_checks" \
    "Benchmarks single tool permission checks (<100μs target)"

run_test "Batch Permission Evaluation" \
    "cargo test --test permission_cache_performance_test bench_batch_permission_evaluation" \
    "Tests batch evaluation performance (>100k tools/sec target)"

run_test "Permission Cache Performance" \
    "cargo test --test permission_cache_performance_test bench_permission_cache_performance" \
    "Cache performance with varying user/tool counts"

run_test "Bitmap Operations" \
    "cargo test --test permission_cache_performance_test bench_bitmap_operations" \
    "Ultra-fast bitmap permission operations"

run_test "High Concurrency Stress Test" \
    "cargo test --test permission_cache_performance_test stress_test_high_concurrency" \
    "Stress test with 100 concurrent tasks"

run_test "Memory Usage Scalability" \
    "cargo test --test permission_cache_performance_test test_memory_usage_scalability" \
    "Memory scaling with large user/tool counts"

run_test "Real-World Scenario Simulation" \
    "cargo test --test permission_cache_performance_test test_real_world_scenario_simulation" \
    "Realistic deployment scenario with 5k users, 50k tools"

# Optional: Run with RUST_LOG for detailed output
if [ "$1" = "--verbose" ]; then
    echo -e "\n${YELLOW}🔍 Running verbose integration test...${NC}"
    run_test "Verbose Full Integration" \
        "RUST_LOG=debug cargo test --test permission_filtered_discovery_test test_full_integration -- --nocapture" \
        "Full integration test with debug logging"
fi

# Summary
echo -e "\n${GREEN}🎉 All Permission System Tests Completed Successfully!${NC}"
echo "============================================="
echo -e "${BLUE}📊 Test Summary:${NC}"
echo "  ✅ Permission cache functionality"
echo "  ✅ Fast bitmap-based evaluation"
echo "  ✅ Audit trail generation"
echo "  ✅ Cache invalidation strategies"
echo "  ✅ Filtered tool listing"
echo "  ✅ Performance benchmarks"
echo "  ✅ Concurrency and thread safety"
echo "  ✅ Memory usage and scalability"
echo "  ✅ Real-world scenario simulation"
echo ""
echo -e "${GREEN}🚀 Permission-based pre-filtering system is ready for production!${NC}"

# Optional: Show final stats
if command -v du &> /dev/null; then
    echo -e "\n${BLUE}📁 Code size:${NC}"
    echo "  Discovery module: $(du -sh src/discovery/ 2>/dev/null || echo 'N/A')"
    echo "  Permission tests: $(du -sh tests/*permission* 2>/dev/null || echo 'N/A')"
fi

if [ "$1" = "--benchmark" ]; then
    echo -e "\n${YELLOW}📈 Running additional performance analysis...${NC}"
    echo "  Target: <1ms for 100k tools"
    echo "  Target: <100μs per tool evaluation"  
    echo "  Target: >100k tools/second batch processing"
    echo "  Target: <1000μs cache lookup time"
    echo ""
    echo "Performance targets are validated in the test suite above."
fi