# MagicTunnel Tools

This directory contains various tools and utilities for MagicTunnel development, testing, and operation.

## Directory Structure

```
tools/
├── validation/      # YAML validation and compliance tools
├── migration/       # YAML format migration tools  
├── integrations/    # External service integration tools
├── testing/         # Development and testing utilities
├── release/         # Release and version management tools
└── README.md        # This file
```

## Tool Categories

### 🔍 Validation Tools (`validation/`)

Tools for validating YAML files and ensuring MCP 2025-06-18 compliance.

- **`validate_yaml_migration.py`** - Core YAML validation with detailed reporting
- **`validate_yaml_migration.sh`** - Shell wrapper with colored output and summary

### 🔄 Migration Tools (`migration/`)

Tools for migrating between YAML formats and versions.

- **`migrate_yaml_to_enhanced.py`** - Migrate legacy YAML to enhanced MCP 2025-06-18 format

### 🔗 Integration Tools (`integrations/`)

Tools for integrating with external services.

- **`google_sheets_tools.py`** - Google Sheets integration (service account auth)
- **`google_sheets_oauth_tools.py`** - Google Sheets integration (OAuth2 auth)

### 🧪 Testing Tools (`testing/`)

Development and testing utilities.

- **`test_search.py`** - Manual semantic search testing
- **`test_rust_semantic.py`** - MagicTunnel semantic search API testing

### 🚀 Release Tools (`release/`)

Version management and release tools.

- **`update-version.sh`** - Automated version updates across project files

## Usage Guidelines

### Running Validation

```bash
# Validate all YAML files in capabilities directory
python3 tools/validation/validate_yaml_migration.py capabilities/

# Use shell wrapper with colored output
bash tools/validation/validate_yaml_migration.sh capabilities/
```

### Running Migration

```bash
# Migrate a legacy YAML file to enhanced format
python3 tools/migration/migrate_yaml_to_enhanced.py input.yaml output.yaml
```

### Using Integration Tools

```bash
# Google Sheets operations (requires GOOGLE_APPLICATION_CREDENTIALS)
python3 tools/integrations/google_sheets_tools.py read "spreadsheet_id" "Sheet1!A:D"
python3 tools/integrations/google_sheets_tools.py list_spreadsheets

# Google Sheets with OAuth2 (requires credentials.json)
python3 tools/integrations/google_sheets_oauth_tools.py read "spreadsheet_id" "A:D"
```

### Development Testing

```bash
# Test semantic search locally
python3 tools/testing/test_search.py

# Test MagicTunnel semantic API (requires running server)
python3 tools/testing/test_rust_semantic.py
```

### Release Management

```bash
# Update version across all files
bash tools/release/update-version.sh 0.3.1
```

## Environment Requirements

### Python Dependencies

Most tools require Python 3.7+ with specific packages:

```bash
# For validation tools
pip install pyyaml jsonschema

# For Google Sheets tools
pip install google-auth google-auth-oauthlib google-auth-httplib2 google-api-python-client

# For testing tools
pip install requests numpy
```

### Environment Variables

Different tools require different environment variables:

```bash
# Google Sheets (service account)
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account.json"

# Google Sheets (OAuth2)
export GOOGLE_OAUTH_CREDENTIALS_FILE="/path/to/credentials.json"
export GOOGLE_OAUTH_TOKEN_FILE="/path/to/token.pickle"

# Semantic search testing
export OPENAI_API_KEY="your-openai-key"
```

## Integration with Main Project

These tools are integrated into the main project through:

1. **Rust Tests**: Validation tools are called by Rust integration tests
2. **Capability Files**: Google Sheets tools are used by capability definitions
3. **CI/CD**: Validation runs in automated testing pipelines
4. **Development Workflow**: Testing tools assist in development debugging

## Contributing

When adding new tools:

1. Place them in the appropriate category directory
2. Add documentation to this README
3. Include usage examples
4. Add any required dependencies to environment setup
5. Consider integration with existing Rust tests

## Security Notes

- **Google Sheets Tools**: Use service accounts in production, OAuth2 for development
- **Validation Tools**: Safe to run on any YAML files
- **Testing Tools**: May require API keys - never commit these to version control
- **Release Tools**: Modify multiple files - always review changes before committing