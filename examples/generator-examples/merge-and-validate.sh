#!/bin/bash
# Example script to merge and validate capability files
# using the unified generator CLI
#
# This script demonstrates how to:
# 1. Merge multiple capability files into a single file
# 2. Validate the merged file to ensure it's compliant with the MCP specification
# 3. Handle validation results appropriately
#
# Usage:
#   ./merge-and-validate.sh
#
# Prerequisites:
#   - The MCP generator CLI must be built (cargo build --release)
#   - The capability files to merge must already exist
#   - The output directory must be writable

# Exit on any error
set -e

# Configuration
INPUT_FILES="./capabilities/graphql/github-capabilities.yaml,./capabilities/openapi/petstore-capabilities.yaml"
OUTPUT_FILE="./capabilities/merged/merged-capabilities.yaml"
MERGE_STRATEGY="rename"  # Options: keep-first, keep-last, rename, error
OUTPUT_DIR="$(dirname "$OUTPUT_FILE")"

# Check if the generator is built
if [ ! -f "./target/release/mcp-generator" ]; then
    echo "Error: MCP generator not found. Build it with 'cargo build --release'"
    exit 1
fi

# Check if input files exist
IFS=',' read -ra FILES <<< "$INPUT_FILES"
for file in "${FILES[@]}"; do
    if [ ! -f "$file" ]; then
        echo "Error: Input file not found: $file"
        echo "Make sure to run generate-all.sh first to create the capability files."
        exit 1
    fi
done

# Create output directory
echo "Creating output directory: $OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

echo "Merging capability files..."
echo "Input files: $INPUT_FILES"
echo "Output file: $OUTPUT_FILE"
echo "Merge strategy: $MERGE_STRATEGY"

# Perform the merge operation
./target/release/mcp-generator merge \
  --input "$INPUT_FILES" \
  --output "$OUTPUT_FILE" \
  --strategy "$MERGE_STRATEGY"

echo "Merge completed successfully!"

echo "Validating merged capability file..."
echo "Running strict validation to ensure compliance with MCP specification..."

# Validate the merged file
# We use a separate command and check the exit code to handle validation failures
if ./target/release/mcp-generator validate --input "$OUTPUT_FILE" --strict; then
  echo "✓ Validation successful! Merged capability file is valid."
  echo "Merged file: $OUTPUT_FILE"
  echo ""
  echo "Next steps:"
  echo "1. Use the merged capability file with your MCP server:"
  echo "   mcp-server --capability-file $OUTPUT_FILE"
  echo "2. Test the capabilities with an MCP client"
  echo "3. Integrate with your application"
else
  echo "✗ Validation failed! Please check the merged capability file for errors."
  echo "Common issues:"
  echo "- Duplicate tool names (consider using a different merge strategy)"
  echo "- Invalid schema definitions"
  echo "- Missing required fields"
  echo ""
  echo "Try running validation without --strict to see all issues:"
  echo "./target/release/mcp-generator validate --input $OUTPUT_FILE"
  exit 1
fi