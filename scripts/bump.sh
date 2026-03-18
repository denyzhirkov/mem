#!/usr/bin/env bash
# Bump version across the project.
# Usage: ./scripts/bump.sh 1.3.0
set -euo pipefail

if [ -z "${1:-}" ]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 1.3.0"
  exit 1
fi

VERSION="$1"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# 1. Cargo workspace (single source for all Rust crates)
sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" "$ROOT/Cargo.toml"

# 2. package.json (source for frontend __APP_VERSION__)
sed -i '' "s/\"version\": \".*\"/\"version\": \"${VERSION}\"/" "$ROOT/apps/desktop/package.json"

# 3. Commit and tag (tag must point to the commit with the new version)
git add "$ROOT/Cargo.toml" "$ROOT/apps/desktop/package.json"
git commit -m "release v${VERSION}"
git tag "v${VERSION}"

echo "Bumped to v${VERSION}"
echo "  -> Cargo.toml [workspace.package]"
echo "  -> apps/desktop/package.json"
echo "  -> committed and tagged v${VERSION}"
