#!/bin/bash
# YAML Migration Validation Script for MCP 2025-06-18
# Validates both legacy and enhanced YAML capability files

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
CAPABILITIES_DIR="${1:-capabilities}"
STRICT_MODE="${2:-false}"
VERBOSE="${3:-false}"

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo -e "${BLUE}📋 MCP 2025-06-18 YAML Migration Validation${NC}"
echo -e "${BLUE}=============================================${NC}"

# Check if Python validation script exists
PYTHON_VALIDATOR="$SCRIPT_DIR/validate_yaml_migration.py"
if [[ ! -f "$PYTHON_VALIDATOR" ]]; then
    echo -e "${RED}❌ Python validation script not found: $PYTHON_VALIDATOR${NC}"
    exit 1
fi

# Check if capabilities directory exists
CAPABILITIES_PATH="$PROJECT_ROOT/$CAPABILITIES_DIR"
if [[ ! -d "$CAPABILITIES_PATH" ]]; then
    echo -e "${RED}❌ Capabilities directory not found: $CAPABILITIES_PATH${NC}"
    exit 1
fi

echo -e "${BLUE}🔍 Validating capabilities in: $CAPABILITIES_PATH${NC}"

# Step 1: YAML Syntax Validation
echo -e "\n${YELLOW}📝 Step 1: YAML Syntax Validation${NC}"
yaml_files=$(find "$CAPABILITIES_PATH" -name "*.yaml" -o -name "*.yml" | wc -l)
echo -e "${BLUE}Found $yaml_files YAML files${NC}"

syntax_errors=0
for yaml_file in $(find "$CAPABILITIES_PATH" -name "*.yaml" -o -name "*.yml"); do
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "${BLUE}  Checking: $yaml_file${NC}"
    fi
    
    # Check YAML syntax using Python
    if ! python3 -c "
import yaml
import sys
try:
    with open('$yaml_file', 'r') as f:
        yaml.safe_load(f)
except yaml.YAMLError as e:
    print('YAML Error in $yaml_file:', e)
    sys.exit(1)
except Exception as e:
    print('Error reading $yaml_file:', e)
    sys.exit(1)
" 2>/dev/null; then
        echo -e "${RED}  ❌ Syntax error in: $yaml_file${NC}"
        syntax_errors=$((syntax_errors + 1))
    else
        if [[ "$VERBOSE" == "true" ]]; then
            echo -e "${GREEN}  ✅ Valid YAML: $yaml_file${NC}"
        fi
    fi
done

if [[ $syntax_errors -gt 0 ]]; then
    echo -e "${RED}❌ Found $syntax_errors YAML syntax errors${NC}"
    exit 1
else
    echo -e "${GREEN}✅ All YAML files have valid syntax${NC}"
fi

# Step 2: MCP 2025-06-18 Compliance Validation
echo -e "\n${YELLOW}🔬 Step 2: MCP 2025-06-18 Compliance Validation${NC}"

# Prepare Python validator arguments
validator_args="$CAPABILITIES_PATH"
if [[ "$STRICT_MODE" == "true" ]]; then
    validator_args="$validator_args --strict"
fi

# Run the Python validator
if python3 "$PYTHON_VALIDATOR" $validator_args; then
    echo -e "${GREEN}✅ MCP 2025-06-18 compliance validation passed${NC}"
    compliance_passed=true
else
    echo -e "${RED}❌ MCP 2025-06-18 compliance validation failed${NC}"
    compliance_passed=false
fi

# Step 3: Format Distribution Analysis
echo -e "\n${YELLOW}📊 Step 3: Format Distribution Analysis${NC}"

legacy_count=0
enhanced_count=0

for yaml_file in $(find "$CAPABILITIES_PATH" -name "*.yaml" -o -name "*.yml"); do
    # Check if file has enhanced format indicators
    if grep -q "classification:" "$yaml_file" || \
       grep -q "discovery_metadata:" "$yaml_file" || \
       grep -q "mcp_capabilities:" "$yaml_file" || \
       grep -q "core:" "$yaml_file" || \
       grep -q "execution:" "$yaml_file"; then
        enhanced_count=$((enhanced_count + 1))
        if [[ "$VERBOSE" == "true" ]]; then
            echo -e "${GREEN}  📄 Enhanced format: $(basename "$yaml_file")${NC}"
        fi
    else
        legacy_count=$((legacy_count + 1))
        if [[ "$VERBOSE" == "true" ]]; then
            echo -e "${BLUE}  📄 Legacy format: $(basename "$yaml_file")${NC}"
        fi
    fi
done

total_files=$((legacy_count + enhanced_count))
enhanced_percentage=$(( (enhanced_count * 100) / total_files ))

echo -e "${BLUE}Format Distribution:${NC}"
echo -e "  Legacy format: $legacy_count files"
echo -e "  Enhanced MCP 2025-06-18 format: $enhanced_count files"
echo -e "  Migration progress: ${enhanced_percentage}%"

# Step 4: Tool Count Analysis
echo -e "\n${YELLOW}🧮 Step 4: Tool Count Analysis${NC}"

total_tools=0
enhanced_tools=0
legacy_tools=0

for yaml_file in $(find "$CAPABILITIES_PATH" -name "*.yaml" -o -name "*.yml"); do
    # Count tools in each file
    file_tools=$(python3 -c "
import yaml
import sys
try:
    with open('$yaml_file', 'r') as f:
        content = yaml.safe_load(f)
    tools = content.get('tools', [])
    print(len(tools))
except:
    print(0)
")
    
    total_tools=$((total_tools + file_tools))
    
    # Determine if enhanced format
    if grep -q "classification:" "$yaml_file" || \
       grep -q "discovery_metadata:" "$yaml_file" || \
       grep -q "mcp_capabilities:" "$yaml_file"; then
        enhanced_tools=$((enhanced_tools + file_tools))
    else
        legacy_tools=$((legacy_tools + file_tools))
    fi
done

echo -e "${BLUE}Tool Count Analysis:${NC}"
echo -e "  Total tools: $total_tools"
echo -e "  Legacy format tools: $legacy_tools"
echo -e "  Enhanced format tools: $enhanced_tools"

# Step 5: Security Analysis
echo -e "\n${YELLOW}🔒 Step 5: Security Configuration Analysis${NC}"

security_configured=0
for yaml_file in $(find "$CAPABILITIES_PATH" -name "*.yaml" -o -name "*.yml"); do
    if grep -q "security:" "$yaml_file" || \
       grep -q "classification:" "$yaml_file" || \
       grep -q "sandbox:" "$yaml_file"; then
        security_configured=$((security_configured + 1))
    fi
done

echo -e "${BLUE}Security Configuration:${NC}"
echo -e "  Files with security config: $security_configured / $total_files"

if [[ $security_configured -gt 0 ]]; then
    echo -e "${GREEN}  ✅ Security configurations detected${NC}"
else
    echo -e "${YELLOW}  ⚠️  No security configurations found${NC}"
fi

# Final Summary
echo -e "\n${BLUE}📋 VALIDATION SUMMARY${NC}"
echo -e "${BLUE}=====================${NC}"

if [[ $syntax_errors -eq 0 ]] && [[ "$compliance_passed" == "true" ]]; then
    echo -e "${GREEN}🎉 ALL VALIDATIONS PASSED${NC}"
    echo -e "${GREEN}✅ YAML syntax: Valid${NC}"
    echo -e "${GREEN}✅ MCP 2025-06-18 compliance: Valid${NC}"
    echo -e "${BLUE}📊 Migration progress: ${enhanced_percentage}% (${enhanced_count}/${total_files} files)${NC}"
    echo -e "${BLUE}🧮 Total tools: $total_tools${NC}"
    exit 0
else
    echo -e "${RED}❌ VALIDATION FAILED${NC}"
    if [[ $syntax_errors -gt 0 ]]; then
        echo -e "${RED}❌ YAML syntax errors: $syntax_errors${NC}"
    fi
    if [[ "$compliance_passed" != "true" ]]; then
        echo -e "${RED}❌ MCP 2025-06-18 compliance: Failed${NC}"
    fi
    exit 1
fi