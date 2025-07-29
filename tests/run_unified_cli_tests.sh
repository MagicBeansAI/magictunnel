#!/bin/bash
# Comprehensive test script for the unified capability generation CLI
# This script runs all the tests and examples for the unified CLI

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print section header
print_header() {
    echo -e "\n${BLUE}==== $1 ====${NC}\n"
}

# Print success message
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Print warning message
print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Print error message
print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Create temporary directory for test files
TEMP_DIR=$(mktemp -d)
echo "Using temporary directory: $TEMP_DIR"

# Ensure temp directory is cleaned up on exit
trap "rm -rf $TEMP_DIR" EXIT

# Build the CLI
print_header "Building the unified CLI"
cargo build --bin mcp-generator
print_success "Build completed"

# Run unit tests
print_header "Running unit tests"
cargo test --lib -- --nocapture
print_success "Unit tests completed"

# Run integration tests for the unified CLI
print_header "Running integration tests for the unified CLI"
cargo test --test unified_cli_config_test -- --nocapture
cargo test --test generator_adapters_test -- --nocapture
cargo test --test capability_merge_test -- --nocapture
cargo test --test capability_validation_test -- --nocapture
cargo test --test unified_cli_integration_test -- --nocapture
print_success "Integration tests completed"

# Create test files for CLI examples
print_header "Creating test files for CLI examples"

# Create a simple GraphQL schema
cat > "$TEMP_DIR/schema.graphql" << EOF
type Query {
  hello: String
  user(id: ID!): User
}

type User {
  id: ID!
  name: String!
  email: String
}
EOF
print_success "Created GraphQL schema"

# Create a simple gRPC proto file
cat > "$TEMP_DIR/service.proto" << EOF
syntax = "proto3";

package test;

service UserService {
  rpc GetUser (GetUserRequest) returns (User);
  rpc ListUsers (ListUsersRequest) returns (ListUsersResponse);
}

message GetUserRequest {
  string user_id = 1;
}

message User {
  string user_id = 1;
  string name = 2;
  string email = 3;
}

message ListUsersRequest {
  int32 page_size = 1;
  int32 page_token = 2;
}

message ListUsersResponse {
  repeated User users = 1;
  string next_page_token = 2;
}
EOF
print_success "Created gRPC proto file"

# Create a simple OpenAPI spec
cat > "$TEMP_DIR/openapi.yaml" << EOF
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /users:
    get:
      operationId: listUsers
      summary: List users
      responses:
        '200':
          description: A list of users
          content:
            application/json:
              schema:
                type: array
                items:
                  \$ref: '#/components/schemas/User'
    post:
      operationId: createUser
      summary: Create a user
      requestBody:
        content:
          application/json:
            schema:
              \$ref: '#/components/schemas/User'
      responses:
        '201':
          description: User created
components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: string
        name:
          type: string
        email:
          type: string
EOF
print_success "Created OpenAPI spec"

# Create a configuration file
cat > "$TEMP_DIR/generator_config.yaml" << EOF
# Global settings for all generators
global:
  tool_prefix: "test"
  output_dir: "$TEMP_DIR/capabilities"

# GraphQL generator settings
graphql:
  endpoint: "https://example.com/graphql"
  schema: "$TEMP_DIR/schema.graphql"
  tool_prefix: "graphql"

# gRPC generator settings
grpc:
  endpoint: "localhost:50051"
  proto: "$TEMP_DIR/service.proto"
  tool_prefix: "grpc"

# OpenAPI generator settings
openapi:
  base_url: "https://example.com/api"
  spec: "$TEMP_DIR/openapi.yaml"
  tool_prefix: "api"

# Output settings
output:
  format: "json"
  pretty: true
EOF
print_success "Created configuration file"

# Create output directory
mkdir -p "$TEMP_DIR/capabilities"
print_success "Created output directory"

# Run CLI examples
print_header "Running CLI examples"

# Test help command
echo "Testing help command..."
./target/debug/mcp-generator --help
print_success "Help command works"

# Test version command
echo "Testing version command..."
./target/debug/mcp-generator --version
print_success "Version command works"

# Test GraphQL generator
echo "Testing GraphQL generator..."
./target/debug/mcp-generator graphql \
  --endpoint "https://example.com/graphql" \
  --schema "$TEMP_DIR/schema.graphql" \
  --output "$TEMP_DIR/capabilities/graphql_tools.json" \
  --prefix "graphql"
print_success "GraphQL generator works"

# Test gRPC generator
echo "Testing gRPC generator..."
./target/debug/mcp-generator grpc \
  --endpoint "localhost:50051" \
  --proto "$TEMP_DIR/service.proto" \
  --output "$TEMP_DIR/capabilities/grpc_tools.json" \
  --prefix "grpc"
print_success "gRPC generator works"

# Test OpenAPI generator
echo "Testing OpenAPI generator..."
./target/debug/mcp-generator openapi \
  --base-url "https://example.com/api" \
  --spec "$TEMP_DIR/openapi.yaml" \
  --output "$TEMP_DIR/capabilities/openapi_tools.json" \
  --prefix "api"
print_success "OpenAPI generator works"

# Test configuration file
echo "Testing configuration file..."
./target/debug/mcp-generator --config "$TEMP_DIR/generator_config.yaml" graphql
print_success "Configuration file works"

# Test merge command
echo "Testing merge command..."
./target/debug/mcp-generator merge \
  --input "$TEMP_DIR/capabilities/graphql_tools.json" \
  --input "$TEMP_DIR/capabilities/grpc_tools.json" \
  --output "$TEMP_DIR/capabilities/merged_tools.json" \
  --strategy "rename"
print_success "Merge command works"

# Test validate command
echo "Testing validate command..."
./target/debug/mcp-generator validate \
  --input "$TEMP_DIR/capabilities/graphql_tools.json" \
  --level "normal"
print_success "Validate command works"

# Verify output files
print_header "Verifying output files"
ls -la "$TEMP_DIR/capabilities/"
print_success "Output files created"

# Print summary
print_header "Test Summary"
echo "All tests and examples completed successfully!"
echo "Generated capability files are in: $TEMP_DIR/capabilities/"
echo "You can inspect these files to see the generated capabilities."
echo "Note: Some tests may have shown warnings, which is expected for test scenarios."

# Exit with success
exit 0