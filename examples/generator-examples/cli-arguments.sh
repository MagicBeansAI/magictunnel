#!/bin/bash
# Example script demonstrating how to use the unified generator CLI
# with command-line arguments instead of configuration files

# Create output directories
mkdir -p ./capabilities/graphql
mkdir -p ./capabilities/grpc
mkdir -p ./capabilities/openapi

# Generate capabilities from GraphQL schema
echo "Generating capabilities from GraphQL schema..."
./target/release/mcp-generator graphql \
  --schema "./data/GraphQLSchema.graphql" \
  --endpoint "https://api.github.com/graphql" \
  --output "./capabilities/graphql/github-capabilities.yaml" \
  --prefix "github" \
  --auth-type "bearer" \
  --auth-token "YOUR_GITHUB_TOKEN"

# Generate capabilities from gRPC protobuf
echo "Generating capabilities from gRPC protobuf..."
./target/release/mcp-generator grpc \
  --proto "./data/grpc_test/comprehensive_test_service.proto" \
  --endpoint "localhost:50051" \
  --output "./capabilities/grpc/grpc-capabilities.yaml" \
  --prefix "grpc" \
  --server-streaming "polling" \
  --client-streaming "agent-level" \
  --bidirectional-streaming "pagination" \
  --include-method-options

# Generate capabilities from OpenAPI specification
echo "Generating capabilities from OpenAPI specification..."
./target/release/mcp-generator openapi \
  --spec "./data/petstore_openapi3.json" \
  --base-url "https://petstore.swagger.io/v2" \
  --output "./capabilities/openapi/petstore-capabilities.yaml" \
  --prefix "petstore" \
  --naming "operation-id" \
  --methods "GET,POST,PUT,DELETE"

echo "All capability files generated successfully!"
echo "Files can be found in the ./capabilities directory."