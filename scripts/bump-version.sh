#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [ -z "${1:-}" ]; then
  echo "Usage: ./scripts/bump-version.sh <version>"
  echo "Example: ./scripts/bump-version.sh 0.2.0"
  exit 1
fi

NEW="$1"

if ! echo "$NEW" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+'; then
  echo "Error: version must be semver (e.g. 0.2.0)"
  exit 1
fi

echo ""
echo "  Bumping to v${NEW}"
echo "  ─────────────────"

# All Cargo.toml files — replace only [package] version (first occurrence)
find "$ROOT/crates" "$ROOT/apps/desktop/src-tauri" -name "Cargo.toml" | while read -r f; do
  # Replace only the version line in [package] section (lines 1-5 typically)
  awk -v new="$NEW" '
    /^\[package\]/ { in_pkg=1 }
    /^\[/ && !/^\[package\]/ { in_pkg=0 }
    in_pkg && /^version = "/ { sub(/version = "[^"]+"/, "version = \"" new "\""); in_pkg=0 }
    { print }
  ' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
  echo "  Cargo: $(echo "$f" | sed "s|$ROOT/||")"
done

# Also update internal workspace dependency versions (mem-* crates)
find "$ROOT/crates" "$ROOT/apps/desktop/src-tauri" -name "Cargo.toml" | while read -r f; do
  sed -i '' -E 's/(mem-[a-z]+.*version = ")[0-9]+\.[0-9]+\.[0-9]+/\1'"$NEW"'/g' "$f"
done

# tauri.conf.json
TAURI_CONF="$ROOT/apps/desktop/src-tauri/tauri.conf.json"
sed -i '' -E 's/"version": "[0-9]+\.[0-9]+\.[0-9]+"/"version": "'"$NEW"'"/' "$TAURI_CONF"
echo "  Tauri: apps/desktop/src-tauri/tauri.conf.json"

# package.json
PKG_JSON="$ROOT/apps/desktop/package.json"
sed -i '' -E 's/"version": "[0-9]+\.[0-9]+\.[0-9]+"/"version": "'"$NEW"'"/' "$PKG_JSON"
echo "  NPM:   apps/desktop/package.json"

# Verify
echo ""
cargo check --manifest-path "$ROOT/Cargo.toml" 2>&1 | tail -1
echo ""
echo "  Done. Next steps:"
echo ""
echo "    git add -A"
echo "    git commit -m \"release v${NEW}\""
echo "    git tag v${NEW}"
echo "    git push origin master --tags"
echo ""
