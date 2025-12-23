#!/bin/bash
# Script to update version numbers in website files for cache busting
# Usage: ./update-version.sh [version]
# If version is not provided, extracts from Cargo.toml or uses git tag

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Try to get version from argument, Cargo.toml, or git tag
if [ -n "$1" ]; then
    VERSION="$1"
elif [ -f "../Cargo.toml" ]; then
    VERSION=$(grep "^version" ../Cargo.toml | head -1 | sed 's/.*= "\(.*\)"/\1/' | tr -d ' ')
elif git describe --tags --exact-match HEAD 2>/dev/null | grep -q "^v"; then
    VERSION=$(git describe --tags --exact-match HEAD | sed 's/^v//')
else
    echo "Error: Could not determine version. Please provide it as an argument."
    echo "Usage: $0 [version]"
    echo "Example: $0 0.1.3"
    exit 1
fi

echo "Updating website files with version $VERSION..."

# Update index.html - replace any existing version query string
if [ -f "index.html" ]; then
    # Use a more robust sed pattern that handles both with and without existing version
    sed -i.bak \
        -e "s|href=\"styles\.css\(?v=[0-9.]*\)*\"|href=\"styles.css?v=$VERSION\"|g" \
        -e "s|src=\"script\.js\(?v=[0-9.]*\)*\"|src=\"script.js?v=$VERSION\"|g" \
        index.html

    # Remove backup files
    rm -f index.html.bak

    echo "✅ Updated version to $VERSION in index.html"
else
    echo "⚠️  Warning: index.html not found in current directory"
    exit 1
fi

