# Version Management

This document explains how version information is managed across the MagicTunnel codebase to ensure consistency and reduce maintenance overhead.

## Problem

Previously, the project name "MagicTunnel" and version numbers were hardcoded in many places:
- Documentation files (README.md, CLAUDE.md, etc.)
- Configuration files (config.yaml.template, magictunnel-config.yaml)
- Source code comments and CLI descriptions
- Build files and scripts

This created maintenance overhead and potential inconsistencies when updating versions.

## Solution

We now use **Cargo.toml as the single source of truth** for project name and version information, with automated tools to keep everything in sync.

## Tools Available

### 1. Build Script (`build.rs`)
Automatically updates key files during compilation:
- `test-resources/info.json`
- `config.yaml.template`
- `magictunnel-config.yaml`

### 2. Version Manager CLI (`cargo run --bin version-manager`)
```bash
# Update all files with current Cargo.toml version
cargo run --bin version-manager -- update

# Check for version inconsistencies
cargo run --bin version-manager -- check

# Set a new version across all files
cargo run --bin version-manager -- set 0.3.0
```

### 3. Shell Script (`scripts/update-version.sh`)
```bash
# Update to current Cargo.toml version
./scripts/update-version.sh

# Update to specific version
./scripts/update-version.sh 0.3.0
```

### 4. Makefile Targets
```bash
# Update version across all files
make update-version

# Check version consistency
make check-version

# Set new version
make set-version VERSION=0.3.0
```

## Source Code Best Practices

### ✅ Use Dynamic References
```rust
// Good - uses Cargo.toml values
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(about = env!("CARGO_PKG_DESCRIPTION"))]
#[command(version)] // Automatically uses CARGO_PKG_VERSION

info!("Starting {} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
```

### ❌ Avoid Hardcoded Values
```rust
// Bad - hardcoded values
#[command(name = "magictunnel")]
#[command(about = "MagicTunnel - Intelligent bridge...")]

info!("Starting MagicTunnel v0.2.48");
```

## Configuration Files

Configuration templates use placeholders that get replaced by the build script:
```yaml
# config.yaml.template
mcp_client:
  protocol_version: "2025-03-26"
  client_name: "magictunnel"        # Updated by build script
  client_version: "0.2.48"         # Updated by build script
```

## Release Process

When releasing a new version:

1. **Update Cargo.toml version**:
   ```bash
   # Edit Cargo.toml manually or use:
   make set-version VERSION=0.3.0
   ```

2. **Verify all files are updated**:
   ```bash
   make check-version
   ```

3. **Update CHANGELOG.md manually** (this is not automated)

4. **Commit and tag**:
   ```bash
   git add .
   git commit -m "Release v0.3.0"
   git tag v0.3.0
   ```

## Files Managed Automatically

- ✅ `test-resources/info.json`
- ✅ `config.yaml.template`
- ✅ `magictunnel-config.yaml`
- ✅ `README.md`
- ✅ `CLAUDE.md`
- ✅ `test-resources/documentation.md`
- ✅ Source code CLI descriptions (via env! macros)

## Files Requiring Manual Updates

- ❌ `CHANGELOG.md` - Requires manual content updates
- ❌ Git tags - Must be created manually
- ❌ Release notes - Must be written manually

## Troubleshooting

### Version Inconsistencies
```bash
# Check what's inconsistent
make check-version

# Fix inconsistencies
make update-version
```

### Build Script Not Running
The build script runs automatically during `cargo build`. If changes aren't applied:
```bash
# Force rebuild
cargo clean
cargo build
```

### Manual Override
If you need to manually update a specific file:
```bash
# Use the version manager for specific operations
cargo run --bin version-manager -- update
```

This centralized approach ensures version consistency across the entire codebase while minimizing manual maintenance overhead.
