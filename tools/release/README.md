# Release Tools

Version management and release automation tools.

## Tools

### `update-version.sh`

**Purpose**: Automated version updates across all project files.

**Usage**:
```bash
bash tools/release/update-version.sh <new_version> [--dry-run] [--commit]
```

**Parameters**:
- `new_version`: Target version (e.g., "0.3.1", "1.0.0-beta.1")
- `--dry-run`: Preview changes without making them
- `--commit`: Automatically commit changes after update

**Features**:
- **Multi-file Updates**: Updates version in all relevant project files
- **Format Validation**: Validates semantic versioning format
- **Backup Creation**: Creates backups before making changes
- **Git Integration**: Optional automatic commit creation
- **Rollback Support**: Easy rollback if issues occur
- **Change Verification**: Shows what will be changed before applying

**Files Updated**:
1. `Cargo.toml` - Main Rust package version
2. `CHANGELOG.md` - Version history and release notes
3. `README.md` - Version badges and documentation
4. `frontend/package.json` - Frontend package version
5. `magictunnel-config.yaml` - Configuration file version
6. `docs/README.md` - Documentation version references

**Example Usage**:
```bash
# Preview version update
bash tools/release/update-version.sh 0.3.1 --dry-run

# Update version
bash tools/release/update-version.sh 0.3.1

# Update and commit automatically
bash tools/release/update-version.sh 0.3.1 --commit
```

**Sample Output**:
```bash
🚀 MagicTunnel Version Update Tool
=================================

Current version: 0.3.0
Target version:  0.3.1

📋 Files to update:
✓ Cargo.toml
✓ CHANGELOG.md  
✓ README.md
✓ frontend/package.json
✓ magictunnel-config.yaml
✓ docs/README.md

🔍 Preview of changes:
--------------------------------------
Cargo.toml:
  - version = "0.3.0"
  + version = "0.3.1"

CHANGELOG.md:
  + ## [0.3.1] - 2024-12-03
  + ### Added
  + - Version update to 0.3.1

README.md:
  - **Current Version**: 0.3.0
  + **Current Version**: 0.3.1

[Additional file changes...]

✅ Proceed with version update? (y/N): y

📝 Creating backups...
✓ Backup created: .version-backup-20241203-143022/

🔄 Updating files...
✓ Updated Cargo.toml
✓ Updated CHANGELOG.md
✓ Updated README.md
✓ Updated frontend/package.json
✓ Updated magictunnel-config.yaml
✓ Updated docs/README.md

✅ Version successfully updated to 0.3.1!

📋 Next steps:
1. Review the changes: git diff
2. Test the build: cargo build --release
3. Update CHANGELOG.md with release notes
4. Commit changes: git add . && git commit -m "chore: bump version to 0.3.1"
5. Create release tag: git tag v0.3.1
6. Push changes: git push && git push --tags
```

## Release Workflow

### Complete Release Process

```bash
#!/bin/bash
# complete-release.sh - Full release automation

set -e

NEW_VERSION="$1"
if [ -z "$NEW_VERSION" ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

echo "🚀 Starting release process for version $NEW_VERSION"

# 1. Update version across files
echo "Step 1: Updating version numbers..."
bash tools/release/update-version.sh "$NEW_VERSION"

# 2. Run tests
echo "Step 2: Running tests..."
cargo test
npm test --prefix frontend

# 3. Run validation
echo "Step 3: Validating configuration..."
bash tools/validation/validate_yaml_migration.sh capabilities/

# 4. Build release
echo "Step 4: Building release..."
cargo build --release
npm run build --prefix frontend

# 5. Update changelog
echo "Step 5: Updating changelog..."
# (Manual step - prompt user to update CHANGELOG.md)
echo "Please update CHANGELOG.md with release notes for version $NEW_VERSION"
echo "Press Enter when done..."
read

# 6. Commit changes
echo "Step 6: Committing changes..."
git add .
git commit -m "chore: release version $NEW_VERSION"

# 7. Create tag
echo "Step 7: Creating git tag..."
git tag "v$NEW_VERSION"

# 8. Push to repository
echo "Step 8: Pushing changes..."
git push origin main
git push origin "v$NEW_VERSION"

echo "✅ Release $NEW_VERSION completed successfully!"
```

### Pre-release Checklist

```bash
#!/bin/bash
# pre-release-checklist.sh

echo "📋 Pre-release Checklist"
echo "========================"

# Check git status
if ! git diff --quiet; then
    echo "❌ Uncommitted changes detected"
    echo "   Please commit or stash changes before release"
    exit 1
fi
echo "✅ Git working directory clean"

# Check branch
current_branch=$(git branch --show-current)
if [ "$current_branch" != "main" ] && [ "$current_branch" != "master" ]; then
    echo "❌ Not on main/master branch (currently on: $current_branch)"
    echo "   Please switch to main/master before release"
    exit 1
fi
echo "✅ On main branch"

# Check tests
echo "🧪 Running tests..."
if ! cargo test --quiet; then
    echo "❌ Tests failed"
    exit 1
fi
echo "✅ All tests passing"

# Check build
echo "🔨 Checking build..."
if ! cargo build --release --quiet; then
    echo "❌ Build failed"
    exit 1
fi
echo "✅ Release build successful"

# Check frontend
if [ -d "frontend" ]; then
    echo "🎨 Checking frontend..."
    if ! npm test --prefix frontend --silent; then
        echo "❌ Frontend tests failed"
        exit 1
    fi
    echo "✅ Frontend tests passing"
fi

# Check documentation
echo "📚 Checking documentation..."
if ! bash tools/validation/validate_yaml_migration.sh capabilities/ false false; then
    echo "❌ Configuration validation failed"
    exit 1
fi
echo "✅ Configuration validation passed"

echo ""
echo "🎉 All pre-release checks passed!"
echo "Ready for version update and release."
```

### Version Format Validation

The update script supports semantic versioning with these formats:

- **Standard**: `1.0.0`, `2.1.3`, `0.15.7`
- **Pre-release**: `1.0.0-alpha.1`, `2.0.0-beta.2`, `3.0.0-rc.1`
- **Build metadata**: `1.0.0+20241203`, `2.0.0-beta.1+exp.sha.5114f85`

```bash
# Valid version examples
bash tools/release/update-version.sh 1.0.0
bash tools/release/update-version.sh 2.1.0-beta.1
bash tools/release/update-version.sh 3.0.0-rc.1+build.123

# Invalid versions (will be rejected)
bash tools/release/update-version.sh 1.0      # Missing patch
bash tools/release/update-version.sh v1.0.0   # Should not include 'v' prefix
bash tools/release/update-version.sh 1.0.0.1  # Four-part version not supported
```

## Advanced Features

### Conditional Updates

```bash
# Update only specific files
SPECIFIC_FILES="Cargo.toml,README.md" bash tools/release/update-version.sh 0.3.1

# Skip backup creation
NO_BACKUP=true bash tools/release/update-version.sh 0.3.1

# Custom commit message
COMMIT_MESSAGE="feat: upgrade to version 0.3.1 with new features" \
bash tools/release/update-version.sh 0.3.1 --commit
```

### Rollback Support

```bash
# Rollback to previous version using backup
restore_from_backup() {
    local backup_dir="$1"
    if [ -d "$backup_dir" ]; then
        echo "Restoring from backup: $backup_dir"
        cp -r "$backup_dir"/* .
        echo "✅ Rollback completed"
    else
        echo "❌ Backup directory not found: $backup_dir"
    fi
}

# Usage
restore_from_backup ".version-backup-20241203-143022"
```

### Custom Version Patterns

```bash
# Add custom file patterns to update
CUSTOM_PATTERNS=(
    "src/version.rs:const VERSION: &str = \"VERSION_PLACEHOLDER\";"
    "scripts/install.sh:MAGICTUNNEL_VERSION=\"VERSION_PLACEHOLDER\""
    "docker/Dockerfile:LABEL version=\"VERSION_PLACEHOLDER\""
)

# The script will automatically detect and update these patterns
```

## Integration with CI/CD

### GitHub Actions Release Workflow

```yaml
# .github/workflows/release.yml
name: Release

on:
  workflow_dispatch:
    inputs:
      version:
        description: 'Release version (e.g., 1.0.0)'
        required: true
        type: string

jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
      
    steps:
      - uses: actions/checkout@v3
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
          
      - name: Run pre-release checks
        run: bash tools/release/pre-release-checklist.sh
        
      - name: Update version
        run: bash tools/release/update-version.sh ${{ github.event.inputs.version }}
        
      - name: Run tests
        run: |
          cargo test
          npm test --prefix frontend
          
      - name: Build release
        run: |
          cargo build --release
          npm run build --prefix frontend
          
      - name: Commit and tag
        run: |
          git config user.name "Release Bot"
          git config user.email "noreply@github.com"
          git add .
          git commit -m "chore: release version ${{ github.event.inputs.version }}"
          git tag "v${{ github.event.inputs.version }}"
          git push origin main
          git push origin "v${{ github.event.inputs.version }}"
          
      - name: Create GitHub Release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: v${{ github.event.inputs.version }}
          release_name: Release ${{ github.event.inputs.version }}
          draft: false
          prerelease: false
```

### Automated Release Notes

```bash
#!/bin/bash
# generate-release-notes.sh

VERSION="$1"
PREVIOUS_VERSION=$(git describe --tags --abbrev=0 HEAD^)

echo "# Release Notes for $VERSION"
echo ""
echo "## Changes since $PREVIOUS_VERSION"
echo ""

# Generate changelog from git commits
echo "### Commits"
git log --pretty=format:"- %s (%h)" "$PREVIOUS_VERSION"..HEAD

echo ""
echo ""

# Generate file changes summary
echo "### Files Changed"
git diff --name-only "$PREVIOUS_VERSION"..HEAD | head -20

echo ""
echo ""

# Generate contributor list
echo "### Contributors"
git log --pretty=format:"%an" "$PREVIOUS_VERSION"..HEAD | sort -u
```

## Error Handling

### Common Issues

1. **Version Format Invalid**
   ```
   Error: Invalid version format: 1.0
   Fix: Use semantic versioning (e.g., 1.0.0)
   ```

2. **File Backup Failed**
   ```
   Error: Cannot create backup directory
   Fix: Check disk space and permissions
   ```

3. **Git Repository Dirty**
   ```
   Error: Uncommitted changes detected
   Fix: Commit or stash changes before version update
   ```

4. **Version Already Exists**
   ```
   Error: Version 1.0.0 already exists as git tag
   Fix: Use a different version number
   ```

### Recovery Procedures

```bash
# Restore from backup if update fails
if [ -d ".version-backup-$(date +%Y%m%d)*" ]; then
    latest_backup=$(ls -td .version-backup-* | head -1)
    cp -r "$latest_backup"/* .
    echo "Restored from backup: $latest_backup"
fi

# Remove failed git tag
git tag -d v1.0.0

# Reset to previous commit
git reset --hard HEAD~1
```

## Dependencies

```bash
# System requirements
bash --version   # Bash 4.0+
git --version    # Git 2.0+
grep --version   # GNU grep
sed --version    # GNU sed

# Optional for enhanced features
jq --version     # JSON processing
yq --version     # YAML processing
```

## Configuration

### Script Configuration

```bash
# Environment variables for customization
export VERSION_UPDATE_BACKUP=true          # Create backups (default: true)
export VERSION_UPDATE_FILES="custom-list"  # Custom file list
export VERSION_UPDATE_PATTERN="custom"     # Custom version pattern
export VERSION_UPDATE_COMMIT_MSG="custom"  # Custom commit message template
```

### File Pattern Configuration

```bash
# ~/.magictunnel-release-config
# Custom patterns for version updates

[patterns]
rust_version = 'version = "VERSION_PLACEHOLDER"'
frontend_version = '"version": "VERSION_PLACEHOLDER"'
docs_version = 'Version: VERSION_PLACEHOLDER'
config_version = 'version: "VERSION_PLACEHOLDER"'

[files]
include = ["Cargo.toml", "package.json", "README.md"]
exclude = ["target/", "node_modules/", ".git/"]

[git]
auto_commit = false
commit_message = "chore: bump version to VERSION_PLACEHOLDER"
create_tag = true
tag_format = "vVERSION_PLACEHOLDER"
```

## Best Practices

1. **Always Backup**: Never skip backup creation
2. **Test First**: Run all tests before version update
3. **Review Changes**: Use `--dry-run` to preview changes
4. **Clean Repository**: Ensure no uncommitted changes
5. **Update Changelog**: Always update CHANGELOG.md with release notes
6. **Tag Consistently**: Use consistent tag format (v1.0.0)
7. **Document Process**: Keep release notes and process documentation updated

## Troubleshooting

### Debug Mode

```bash
# Enable debug output
export DEBUG_VERSION_UPDATE=true
bash tools/release/update-version.sh 0.3.1

# Verbose git operations
export GIT_TRACE=1
bash tools/release/update-version.sh 0.3.1 --commit
```

### Manual Recovery

```bash
# If automated recovery fails, manual steps:

# 1. Restore files from backup
cp -r .version-backup-TIMESTAMP/* .

# 2. Reset git if needed
git reset --hard HEAD~1

# 3. Remove failed tags
git tag -d v1.0.0

# 4. Clean up backup directories
rm -rf .version-backup-*
```