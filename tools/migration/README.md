# YAML Migration Tools

Tools for migrating between YAML formats and versions.

## Tools

### `migrate_yaml_to_enhanced.py`

**Purpose**: Migrate legacy YAML capability files to enhanced MCP 2025-06-18 format.

**Usage**:
```bash
python3 migrate_yaml_to_enhanced.py <input_file> <output_file>
```

**Parameters**:
- `input_file`: Path to legacy YAML capability file
- `output_file`: Path where enhanced format file will be written

**Features**:
- **Format Detection**: Automatically detects current YAML format
- **Schema Enhancement**: Upgrades to MCP 2025-06-18 enhanced schema
- **Security Integration**: Adds security configuration sections
- **Metadata Preservation**: Maintains existing tool definitions and metadata
- **Validation**: Validates output format compliance
- **Backup Creation**: Optionally creates backup of original file

**Migration Changes**:
1. **Schema Version**: Updates schema version to MCP 2025-06-18
2. **Security Sections**: Adds authentication, authorization, and security policies
3. **Enhanced Routing**: Upgrades routing configurations for new transport types
4. **Metadata Enhancement**: Enriches tool metadata with additional fields
5. **Compliance Features**: Adds monitoring, cancellation, and progress tracking

**Example**:
```bash
# Migrate a legacy capability file
python3 tools/migration/migrate_yaml_to_enhanced.py \
  capabilities/legacy/old_http_client.yaml \
  capabilities/web/http_client.yaml

# Output example
Migrating: capabilities/legacy/old_http_client.yaml
Target: capabilities/web/http_client.yaml

✓ Input validation passed
✓ Legacy format detected
✓ Schema upgrade completed
✓ Security sections added
✓ Enhanced routing configured
✓ Output validation passed
✓ Migration completed successfully

Summary:
  Tools migrated: 12
  Security configs added: 5
  Routing enhancements: 8
  Format: Legacy → Enhanced MCP 2025-06-18
```

## Migration Workflow

### 1. Pre-Migration Assessment

```bash
# Check current format distribution
python3 tools/validation/validate_yaml_migration.py capabilities/

# Identify legacy files
grep -r "version:" capabilities/ | grep -v "2025-06-18"
```

### 2. Batch Migration

```bash
# Migrate all legacy files in a directory
find capabilities/ -name "*.yaml" -type f | while read file; do
    if grep -q "legacy_format" "$file"; then
        output_file="${file%.yaml}_enhanced.yaml"
        python3 tools/migration/migrate_yaml_to_enhanced.py "$file" "$output_file"
    fi
done
```

### 3. Post-Migration Validation

```bash
# Validate all migrated files
python3 tools/validation/validate_yaml_migration.py capabilities/

# Check migration completeness
bash tools/validation/validate_yaml_migration.sh capabilities/ false true
```

## Migration Rules

### Schema Updates

The migration tool applies these transformations:

1. **Version Update**:
   ```yaml
   # Before
   version: "1.0"
   
   # After  
   version: "MCP 2025-06-18"
   format: "enhanced"
   ```

2. **Security Addition**:
   ```yaml
   # Added security section
   security:
     authentication:
       type: "bearer_token"
       required: true
     authorization:
       permissions: ["read", "write"]
     policies:
       rate_limiting: true
       content_filtering: true
   ```

3. **Enhanced Routing**:
   ```yaml
   # Before
   routing:
     type: "http"
     
   # After
   routing:
     type: "enhanced_http"
     transport: "streamable_http"
     features:
       - "streaming"
       - "cancellation"
       - "progress_tracking"
   ```

### Compatibility Preservation

- **Tool Names**: Preserved exactly
- **Parameter Schemas**: Maintained with validation
- **Endpoint URLs**: Kept unchanged
- **Description Text**: Preserved with enhancement
- **Custom Fields**: Migrated when compatible

## Error Handling

### Common Migration Issues

1. **Invalid Input Format**
   ```
   Error: Input file is not valid YAML
   Fix: Check YAML syntax with yamllint
   ```

2. **Unsupported Schema Version**
   ```
   Error: Unknown schema version detected
   Fix: Manually update version field before migration
   ```

3. **Routing Type Conflicts**
   ```
   Error: Cannot enhance routing type 'custom_type'
   Fix: Use supported routing types or manual configuration
   ```

4. **Security Configuration Conflicts**
   ```
   Warning: Existing security config found, merging...
   Result: Manual review recommended for security policies
   ```

## Integration

### With Validation Tools

```bash
# Complete migration and validation workflow
python3 tools/migration/migrate_yaml_to_enhanced.py input.yaml output.yaml
python3 tools/validation/validate_yaml_migration.py $(dirname output.yaml)
```

### With Version Control

```bash
# Create migration branch
git checkout -b yaml-migration

# Migrate files
python3 tools/migration/migrate_yaml_to_enhanced.py capabilities/old.yaml capabilities/new.yaml

# Validate migration
bash tools/validation/validate_yaml_migration.sh capabilities/

# Commit if successful
git add capabilities/
git commit -m "Migrate to MCP 2025-06-18 enhanced format"
```

## Dependencies

```bash
# Python packages
pip install pyyaml jsonschema

# System requirements
python3 --version  # Python 3.7+
```

## Configuration

The migration tool uses these default settings:

```yaml
# Migration configuration (embedded in tool)
migration:
  preserve_comments: true
  backup_original: false
  validate_output: true
  security_defaults:
    authentication_required: true
    rate_limiting_enabled: true
    content_filtering_enabled: true
  enhanced_features:
    enable_streaming: true
    enable_cancellation: true
    enable_progress_tracking: true
```

## Best Practices

1. **Always Backup**: Create backups before migration
2. **Validate First**: Run validation on input files
3. **Test Incrementally**: Migrate one file at a time initially
4. **Review Security**: Manually review added security configurations
5. **Update Documentation**: Update any references to old file paths
6. **Run Tests**: Execute full test suite after migration

## Troubleshooting

### Migration Fails

```bash
# Check input file validity
python3 -c "import yaml; yaml.safe_load(open('input.yaml'))"

# Run with debug output
python3 -v tools/migration/migrate_yaml_to_enhanced.py input.yaml output.yaml
```

### Output Validation Fails

```bash
# Check specific validation errors
python3 tools/validation/validate_yaml_migration.py output.yaml

# Compare with working examples
diff output.yaml capabilities/working_example.yaml
```

### Performance Issues

For large files or batch operations:

```bash
# Process in parallel (requires GNU parallel)
find capabilities/ -name "*.yaml" | parallel python3 tools/migration/migrate_yaml_to_enhanced.py {} {.}_enhanced.yaml
```