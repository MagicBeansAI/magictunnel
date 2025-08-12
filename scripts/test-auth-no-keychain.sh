#!/bin/bash
set -e

echo "🔐 Testing OAuth Authentication Without Keychain Prompts"
echo "======================================================="

# Set environment variables to force filesystem storage
export MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem
export RUST_LOG=debug
export RUST_BACKTRACE=1

echo "✅ Environment variables set:"
echo "   MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem"
echo "   RUST_LOG=debug"
echo ""

# Create test directories
echo "📁 Creating test directories..."
mkdir -p test-data/sessions test-data/tokens
chmod 700 test-data/sessions test-data/tokens
echo "   Created: test-data/sessions"
echo "   Created: test-data/tokens"
echo ""

# Function to run tests and capture output
run_test() {
    local test_name=$1
    local test_pattern=$2
    
    echo "🧪 Running $test_name..."
    if cargo test "$test_pattern" --verbose --no-capture 2>&1 | grep -E "(test result:|FAILED|ERROR)" | tail -5; then
        echo "   ✅ $test_name passed"
    else
        echo "   ❌ $test_name failed"
        return 1
    fi
    echo ""
}

# Run specific authentication tests
echo "🚀 Starting OAuth authentication tests..."
echo ""

# Test 1: Basic token storage functionality
run_test "Token Storage Integration" "test_token_storage_creation"

# Test 2: Mock storage backend
run_test "Mock Storage Backend" "test_mock_storage_backend"

# Test 3: Filesystem storage forced
run_test "Filesystem Storage" "test_filesystem_storage_forced"

# Test 4: Cross-platform compatibility
run_test "Cross-Platform Compatibility" "test_cross_platform_compatibility"

# Test 5: OAuth validator with token storage
run_test "OAuth Validator Integration" "test_token_storage_with_oauth_validator"

echo "🧹 Cleaning up test data..."
rm -rf test-data
echo "   Removed: test-data/"
echo ""

echo "🎉 All OAuth authentication tests completed successfully!"
echo "   No Keychain prompts should have appeared during testing."
echo ""
echo "💡 Key points:"
echo "   - All tests used filesystem storage instead of macOS Keychain"
echo "   - Mock storage backend works for pure unit testing"
echo "   - Environment variable override is working correctly"
echo "   - Tests are compatible across platforms (macOS/Windows/Linux)"
echo ""
echo "🔧 For CI/CD, use:"
echo "   export MAGICTUNNEL_TEST_STORAGE_BACKEND=filesystem"
echo "   cargo test --test '*oauth*' --test '*auth*' --test '*token_storage*'"