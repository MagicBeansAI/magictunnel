# Unified Capability Generation CLI Tests

This directory contains comprehensive tests for the unified capability generation CLI (`mcp-generator`). These tests validate the functionality of the CLI, including configuration parsing, generator adapters, capability merging, and validation.

## Test Files

- `unified_cli_config_test.rs`: Tests for configuration file parsing and validation
- `generator_adapters_test.rs`: Tests for generator adapters (GraphQL, gRPC, OpenAPI)
- `capability_merge_test.rs`: Tests for capability merging functionality
- `capability_validation_test.rs`: Tests for capability validation functionality
- `unified_cli_integration_test.rs`: Integration tests for the end-to-end CLI functionality
- `run_unified_cli_tests.sh`: Shell script to run all tests and examples

## Running the Tests

### Using Cargo

You can run individual test files using Cargo:

```bash
# Run configuration tests
cargo test --test unified_cli_config_test

# Run generator adapter tests
cargo test --test generator_adapters_test

# Run capability merge tests
cargo test --test capability_merge_test

# Run capability validation tests
cargo test --test capability_validation_test

# Run integration tests
cargo test --test unified_cli_integration_test
```

To run all tests with verbose output:

```bash
cargo test -- --nocapture
```

### Using the Test Script

The `run_unified_cli_tests.sh` script provides a comprehensive test suite that:

1. Builds the CLI
2. Runs all unit tests
3. Runs all integration tests
4. Creates test files for CLI examples
5. Runs CLI examples with various commands
6. Verifies output files

To run the script:

```bash
./tests/run_unified_cli_tests.sh
```

## Test Coverage

The tests cover the following aspects of the unified CLI:

### Configuration Parsing and Validation

- TOML configuration file parsing
- Environment variable overrides
- Configuration validation
- Default values
- Generator-specific configuration

### Generator Adapters

- GraphQL generator adapter
- gRPC generator adapter
- OpenAPI generator adapter
- Common interface implementation
- Authentication configuration
- Generator-specific options

### Capability Merging

- Basic merging of capability files
- Conflict resolution strategies (Error, FirstWins, LastWins, Rename)
- Edge cases (empty files, invalid files)
- Schema version compatibility

### Capability Validation

- Schema validation
- Tool definition validation
- Validation levels (Strict, Normal, Relaxed)
- Custom validation rules

### Integration Testing

- Command-line argument parsing
- Configuration file loading
- Generator execution
- Merging functionality
- Validation functionality
- End-to-end workflow

## Adding New Tests

When adding new features to the unified CLI, please add corresponding tests to maintain test coverage. Follow these guidelines:

1. Add unit tests for new functionality
2. Add integration tests for end-to-end workflows
3. Update the test script to include new test cases
4. Document the new tests in this README

## Test Data

The tests use a combination of:

- In-memory test data
- Temporary files created during test execution
- Example files in the `examples` directory

The test script creates temporary files for testing, which are automatically cleaned up after the tests complete.