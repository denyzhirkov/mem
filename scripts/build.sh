#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"
VERSION="${1:-dev}"

echo ""
echo "  mem build v${VERSION}"
echo "  ─────────────────"
echo ""

rm -rf "$DIST"
mkdir -p "$DIST"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
esac
PLATFORM="${OS}-${ARCH}"

# ── CLI ──
echo "[1/2] Building CLI..."
cargo build --release --manifest-path "$ROOT/Cargo.toml" -p mem-cli

CLI_BIN="$ROOT/target/release/mem"
if [ -f "$CLI_BIN" ]; then
  cp "$CLI_BIN" "$DIST/mem-${PLATFORM}"
  chmod +x "$DIST/mem-${PLATFORM}"
  echo "  -> dist/mem-${PLATFORM}"
else
  echo "  ERROR: CLI binary not found"
  exit 1
fi

# ── Desktop ──
echo "[2/2] Building desktop app..."
cd "$ROOT/apps/desktop"
[ -d "node_modules" ] || npm install
npx tauri build 2>&1 | tail -5

BUNDLE_DIR="$ROOT/apps/desktop/src-tauri/target/release/bundle"

if [ "$OS" = "darwin" ]; then
  # .app
  APP=$(find "$BUNDLE_DIR/macos" -name "*.app" -maxdepth 1 | head -1)
  [ -n "$APP" ] && cp -r "$APP" "$DIST/" && echo "  -> dist/$(basename "$APP")"
  # .dmg
  DMG=$(find "$BUNDLE_DIR/dmg" -name "*.dmg" | head -1)
  [ -n "$DMG" ] && cp "$DMG" "$DIST/" && echo "  -> dist/$(basename "$DMG")"
elif [ "$OS" = "linux" ]; then
  AI=$(find "$BUNDLE_DIR/appimage" -name "*.AppImage" 2>/dev/null | head -1)
  [ -n "$AI" ] && cp "$AI" "$DIST/" && echo "  -> dist/$(basename "$AI")"
  DEB=$(find "$BUNDLE_DIR/deb" -name "*.deb" 2>/dev/null | head -1)
  [ -n "$DEB" ] && cp "$DEB" "$DIST/" && echo "  -> dist/$(basename "$DEB")"
fi

# Signatures for updater
find "$BUNDLE_DIR" -name "*.sig" -exec cp {} "$DIST/" \; 2>/dev/null || true

echo ""
echo "  Build complete. Artifacts:"
ls -lh "$DIST/"
echo ""
