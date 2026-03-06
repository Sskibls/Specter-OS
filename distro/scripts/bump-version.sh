#!/bin/bash
# SpecterOS Version Bump Script
# Automatically increments version by 0.0.1

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
BUILD_SCRIPT="$PROJECT_ROOT/distro/live-build/build.sh"

# Get current version
CURRENT_VERSION=$(grep '^VERSION=' "$BUILD_SCRIPT" | head -1 | cut -d'"' -f2)

if [ -z "$CURRENT_VERSION" ]; then
    CURRENT_VERSION="0.2.0"
fi

# Parse version components
MAJOR=$(echo "$CURRENT_VERSION" | cut -d. -f1)
MINOR=$(echo "$CURRENT_VERSION" | cut -d. -f2)
PATCH=$(echo "$CURRENT_VERSION" | cut -d. -f3)

# Increment patch version
NEW_PATCH=$((PATCH + 1))
NEW_VERSION="${MAJOR}.${MINOR}.${NEW_PATCH}"

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║         SpecterOS Version Bump                            ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""
echo "Current version:  $CURRENT_VERSION"
echo "New version:      $NEW_VERSION"
echo ""

# Update version in build.sh
sed -i "s/^VERSION=\"${CURRENT_VERSION}\"/VERSION=\"${NEW_VERSION}\"/" "$BUILD_SCRIPT"

# Update version in all Cargo.toml files
find "$PROJECT_ROOT" -name "Cargo.toml" -type f -exec sed -i "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" {} \;

# Update version in website
sed -i "s/v${CURRENT_VERSION}/v${NEW_VERSION}/g" "$PROJECT_ROOT/website/index.html" 2>/dev/null || true

# Update version in README
sed -i "s/v${CURRENT_VERSION}/v${NEW_VERSION}/g" "$PROJECT_ROOT/README.md" 2>/dev/null || true

# Create git commit
cd "$PROJECT_ROOT"
git add .
git commit -m "Bump version to ${NEW_VERSION}" || echo "No changes to commit"

echo ""
echo "✓ Version bumped to ${NEW_VERSION}"
echo ""
echo "Updated files:"
echo "  - distro/live-build/build.sh"
echo "  - All Cargo.toml files"
echo "  - website/index.html"
echo "  - README.md"
echo ""
echo "Commit created: 'Bump version to ${NEW_VERSION}'"
echo ""
echo "To push to GitHub:"
echo "  git push origin main"
echo ""
