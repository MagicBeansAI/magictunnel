# YAML Validation Tools

Tools for validating YAML capability files and ensuring MCP 2025-06-18 compliance.

## Tools

### `validate_yaml_migration.py`

**Purpose**: Comprehensive YAML validation with detailed compliance reporting.

**Usage**:
```bash
python3 validate_yaml_migration.py <directory_path>
```

**Features**:
- **Format Detection**: Automatically detects legacy vs enhanced MCP 2025-06-18 format
- **Schema Validation**: Validates YAML structure and required fields
- **Compliance Checking**: Ensures MCP 2025-06-18 specification compliance
- **Security Analysis**: Checks for security configuration presence
- **Detailed Reporting**: Color-coded output with errors, warnings, and info messages

**Output Example**:
```
================================================================================
YAML MIGRATION VALIDATION RESULTS
================================================================================

SUMMARY:
  Errors: 0
  Warnings: 4
  Info: 19

[INFO] capabilities
  Found 18 YAML files to validate

[INFO] capabilities/core/file_operations.yaml
  Detected format: Enhanced MCP 2025-06-18

[WARNING] capabilities/system/monitoring.yaml
  Tool 'intelligent_memory_analyzer': Monitoring section missing cancellation.enabled

================================================================================
VALIDATION PASSED WITH WARNINGS: 4 warnings
```

**Exit Codes**:
- `0`: Validation passed (may have warnings)
- `1`: Validation failed with errors

### `validate_yaml_migration.sh`

**Purpose**: Shell wrapper for Python validator with enhanced formatting and summary.

**Usage**:
```bash
bash validate_yaml_migration.sh <directory_path> [strict_mode] [verbose_mode]
```

**Parameters**:
- `directory_path`: Path to capabilities directory
- `strict_mode`: `true` to treat warnings as errors (default: `false`)
- `verbose_mode`: `true` for detailed output (default: `true`)

**Features**:
- **Color-coded Output**: Green for success, red for errors, yellow for warnings
- **Progress Indicators**: Step-by-step validation progress
- **Format Analysis**: Migration progress statistics
- **Tool Counting**: Total tools analysis
- **Security Summary**: Security configuration coverage
- **Integration Ready**: Designed for CI/CD pipelines

**Output Example**:
```
📋 MCP 2025-06-18 YAML Migration Validation
=============================================
🔍 Validating capabilities in: /path/to/capabilities

📝 Step 1: YAML Syntax Validation
Found       18 YAML files
✅ All YAML files have valid syntax

🔬 Step 2: MCP 2025-06-18 Compliance Validation
✅ MCP 2025-06-18 compliance validation passed

📊 Step 3: Format Distribution Analysis
Format Distribution:
  Legacy format: 2 files
  Enhanced MCP 2025-06-18 format: 16 files
  Migration progress: 88%

🧮 Step 4: Tool Count Analysis
Tool Count Analysis:
  Total tools: 99
  Legacy format tools: 23
  Enhanced format tools: 76

🔒 Step 5: Security Configuration Analysis
Security Configuration:
  Files with security config: 16 / 18
  ✅ Security configurations detected

📋 VALIDATION SUMMARY
=====================
🎉 ALL VALIDATIONS PASSED
✅ YAML syntax: Valid
✅ MCP 2025-06-18 compliance: Valid
📊 Migration progress: 88% (16/18 files)
🧮 Total tools: 99
```

## Integration

### Rust Integration Tests

The validation tools are integrated into Rust tests:

```rust
// tests/yaml_migration_validation_test.rs
#[tokio::test]
async fn test_validation_script_integration() {
    let bash_script = get_bash_validation_script();
    
    let output = Command::new("bash")
        .arg(&bash_script)
        .arg("capabilities")
        .arg("false") // non-strict mode
        .arg("false") // non-verbose mode
        .current_dir(&capabilities_dir.parent().unwrap())
        .output()
        .expect("Failed to execute bash validation script");

    assert!(output.status.success());
}
```

### CI/CD Integration

Add to your CI pipeline:

```yaml
- name: Validate YAML Files
  run: |
    bash tools/validation/validate_yaml_migration.sh capabilities/ true false
```

## Dependencies

```bash
# Python packages
pip install pyyaml jsonschema

# System requirements
bash --version  # Bash 4.0+
python3 --version  # Python 3.7+
```

## Configuration

Both tools work with the standard MagicTunnel capabilities directory structure:

```
capabilities/
├── core/
│   └── file_operations.yaml
├── web/
│   └── http_client.yaml
├── google/
│   ├── google_sheets.yaml
│   └── sales_sheets.yaml
└── ...
```

## Error Types

### Errors (Exit code 1)
- Invalid YAML syntax
- Missing required fields
- Invalid schema structure
- MCP specification violations

### Warnings (Exit code 0)
- Missing optional fields
- Deprecated usage patterns
- Security configuration recommendations
- Format upgrade suggestions

### Info Messages
- File discovery results
- Format detection results
- Progress updates
- Summary statistics

## Troubleshooting

### Common Issues

1. **"YAML syntax errors found"**
   - Check for indentation issues
   - Verify quotes and special characters
   - Use YAML linter to identify syntax problems

2. **"Missing required metadata field"**
   - Ensure `name` and `description` fields are present
   - Check that fields are not empty or null

3. **"Invalid routing type"**
   - Use supported routing types: `http`, `grpc`, `graphql`, `websocket`, `subprocess`, `lambda`
   - Or enhanced types with prefixes: `enhanced_*`, `ai_*`, `external_*`

4. **"Path traversal pattern detected"**
   - Remove `..` patterns from configuration paths
   - Use absolute paths or safe relative paths

### Debug Mode

Run with verbose Python output:
```bash
python3 -v tools/validation/validate_yaml_migration.py capabilities/
```

Run bash script in debug mode:
```bash
bash -x tools/validation/validate_yaml_migration.sh capabilities/
```