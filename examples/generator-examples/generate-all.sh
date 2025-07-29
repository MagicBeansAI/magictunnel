#!/bin/bash
# Example script to generate capability files from multiple sources
# using the unified generator CLI
#
# This script demonstrates how to:
# 1. Generate capability files from different API types (GraphQL, gRPC, OpenAPI)
# 2. Use a comprehensive configuration file for all generators
# 3. Organize output files in a structured directory layout
#
# Usage:
#   ./generate-all.sh
#
# Prerequisites:
#   - The MCP generator CLI must be built (cargo build --release)
#   - The example configuration files must be available
#   - The schema/proto/spec files must be available

# Exit on any error
set -e

# Print commands before executing them
set -x

# Configuration and file paths
CONFIG_FILE="./examples/generator-examples/comprehensive-config.yaml"
GRAPHQL_SCHEMA="./data/github_schema.graphql"
GRPC_PROTO="./data/grpc_test/comprehensive_test_service.proto"
OPENAPI_SPEC="./data/petstore_openapi3.json"
OUTPUT_DIR="./capabilities"

# Create output directories
echo "Creating output directories..."
mkdir -p "$OUTPUT_DIR/combined"
mkdir -p "$OUTPUT_DIR/graphql"
mkdir -p "$OUTPUT_DIR/grpc"
mkdir -p "$OUTPUT_DIR/openapi"
mkdir -p "$OUTPUT_DIR/merged"

# Function to check if a file exists
check_file() {
    if [ ! -f "$1" ]; then
        echo "Error: File not found: $1"
        exit 1
    fi
}

# Check if required files exist
check_file "$CONFIG_FILE"
check_file "$GRAPHQL_SCHEMA"
check_file "$GRPC_PROTO"
check_file "$OPENAPI_SPEC"

# Check if the generator is built
if [ ! -f "./target/release/mcp-generator" ]; then
    echo "Error: MCP generator not found. Build it with 'cargo build --release'"
    exit 1
fi

# Generate capabilities from GraphQL schema
echo "Generating capabilities from GraphQL schema..."
./target/release/mcp-generator graphql --config "$CONFIG_FILE" --schema "$GRAPHQL_SCHEMA"
echo "✓ GraphQL capabilities generated"

# Generate capabilities from gRPC protobuf
echo "Generating capabilities from gRPC protobuf..."
./target/release/mcp-generator grpc --config "$CONFIG_FILE" --proto "$GRPC_PROTO"
echo "✓ gRPC capabilities generated"

# Generate capabilities from OpenAPI specification
echo "Generating capabilities from OpenAPI specification..."
./target/release/mcp-generator openapi --config "$CONFIG_FILE" --spec "$OPENAPI_SPEC"
echo "✓ OpenAPI capabilities generated"

echo "All capability files generated successfully!"
echo "Files can be found in the $OUTPUT_DIR directory:"
echo "  - GraphQL: $OUTPUT_DIR/combined/github-capabilities.yaml"
echo "  - gRPC: $OUTPUT_DIR/combined/grpc-capabilities.yaml"
echo "  - OpenAPI: $OUTPUT_DIR/combined/petstore-capabilities.yaml"

echo "Next steps:"
echo "1. Inspect the generated files to ensure they meet your requirements"
echo "2. Merge the files using the merge-and-validate.sh script"
echo "3. Use the merged capability file with your MCP server"