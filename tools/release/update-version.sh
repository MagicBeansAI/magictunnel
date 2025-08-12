#!/bin/bash

# Script to update version information across all documentation files
# Usage: ./scripts/update-version.sh [new_version]

set -e

# Get version from Cargo.toml if not provided
if [ -z "$1" ]; then
    VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
else
    VERSION="$1"
fi

# Get project name from Cargo.toml
PROJECT_NAME=$(grep '^name = ' Cargo.toml | sed 's/name = "\(.*\)"/\1/')
PROJECT_DESCRIPTION=$(grep '^description = ' Cargo.toml | sed 's/description = "\(.*\)"/\1/')

echo "Updating version to: $VERSION"
echo "Project name: $PROJECT_NAME"

# Function to update version in files
update_version_in_file() {
    local file="$1"
    local pattern="$2"
    local replacement="$3"
    local delimiter="$4"
    
    # Use custom delimiter if provided, otherwise default to /
    if [ -n "$delimiter" ]; then
        local sed_cmd="s${delimiter}${pattern}${delimiter}${replacement}${delimiter}g"
    else
        local sed_cmd="s/${pattern}/${replacement}/g"
    fi
    
    if [ -f "$file" ]; then
        echo "Updating $file..."
        sed -i.bak "$sed_cmd" "$file"
        rm -f "$file.bak"
    else
        echo "Warning: $file not found"
    fi
}

# Update Cargo.toml first (most important)
update_version_in_file "Cargo.toml" "version = \"0\.[0-9]\+\.[0-9]\+\"" "version = \"$VERSION\""

# Update documentation files (support both 0.2.x and 0.3.x patterns)
update_version_in_file "README.md" "Current Version\*\*: 0\.[0-9]\+\.[0-9]\+" "Current Version**: $VERSION"
update_version_in_file "CLAUDE.md" "Current Version\*\*: 0\.[0-9]\+\.[0-9]\+" "Current Version**: $VERSION"
update_version_in_file "CLAUDE.md" "Version 0\.[0-9]\+\.[0-9]\+ (Current)" "Version $VERSION (Current)"

# Update configuration files
update_version_in_file "config.yaml.template" "client_version: \"0\.[0-9]\+\.[0-9]\+\"" "client_version: \"$VERSION\""
update_version_in_file "magictunnel-config.yaml" "client_version: \"0\.[0-9]\+\.[0-9]\+\"" "client_version: \"$VERSION\""

# Update JSON files
update_version_in_file "test-resources/info.json" "\"version\": \"0\.[0-9]\+\.[0-9]\+\"" "\"version\": \"$VERSION\""

# Update test files (using simpler patterns)
update_version_in_file "tests/test_config_validation.rs" "0\.[0-9]\+\.[0-9]\+\.to_string" "$VERSION.to_string"
update_version_in_file "tests/mcp_external_tests.rs" "0\.[0-9]\+\.[0-9]\+\.to_string" "$VERSION.to_string"

# Update source files
update_version_in_file "src/mcp/server.rs" "\"version\": \"0\.[0-9]\+\.[0-9]\+\"" "\"version\": \"$VERSION\""
update_version_in_file "src/auth/oauth.rs" "magictunnel/0\.[0-9]\+\.[0-9]\+" "magictunnel/$VERSION" "|"

# Update frontend package.json
update_version_in_file "frontend/package.json" "\"version\": \"0\.0\.[0-9]\+\"" "\"version\": \"$VERSION\""

echo "Version update complete!"
echo "Don't forget to:"
echo "1. Update CHANGELOG.md manually"
echo "2. Commit the changes"
echo "3. Create a git tag: git tag v$VERSION"
