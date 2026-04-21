#!/usr/bin/env bash
# Build mem from source and install locally.
# Usage: ./setup.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64)        ARCH_ID="x86_64";  DEB_ARCH="amd64" ;;
  arm64|aarch64) ARCH_ID="aarch64"; DEB_ARCH="arm64"  ;;
  *) echo "Unsupported arch: $ARCH"; exit 1 ;;
esac
PLATFORM="${OS}-${ARCH_ID}"
VERSION=$(grep '^version' "$ROOT/Cargo.toml" | head -1 | sed -E 's/.*"(.+)".*/\1/')
INSTALL_BIN="${HOME}/.local/bin"

echo ""
echo "  mem setup v${VERSION}"
echo "  ────────────────────"
echo "  Platform: ${PLATFORM}"
echo ""

# ── 1. Rust ──
echo "[1/5] Checking Rust..."
if ! command -v cargo &>/dev/null; then
  echo "  Installing rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
  source "${HOME}/.cargo/env"
else
  echo "  $(rustc --version) ✓"
fi

# ── 2. Node.js ──
echo "[2/5] Checking Node.js..."
if ! command -v node &>/dev/null || ! command -v npm &>/dev/null; then
  echo "  ERROR: Node.js / npm not found."
  echo "  Install: brew install node  OR  https://nodejs.org"
  exit 1
fi
echo "  node $(node --version) / npm $(npm --version) ✓"

# ── 3. System deps (Linux only) ──
echo "[3/5] System dependencies..."
if [ "$OS" = "linux" ]; then
  if command -v apt-get &>/dev/null; then
    echo "  apt: installing Tauri deps..."
    sudo apt-get update -qq
    sudo apt-get install -y -qq \
      libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf file
  else
    echo "  Non-apt Linux: install manually — libwebkit2gtk-4.1, libappindicator3, librsvg2"
  fi
else
  echo "  macOS — no extra deps ✓"
fi

# ── 4. Build ──
echo "[4/5] Building..."

echo "  [4a] CLI..."
cargo build --release --manifest-path "$ROOT/Cargo.toml" -p mem-cli

echo "  [4b] MCP server..."
cargo build --release --manifest-path "$ROOT/Cargo.toml" -p mem-mcp

echo "  [4c] Desktop app..."
cd "$ROOT/apps/desktop"
[ -d "node_modules" ] || npm install --silent
npx tauri build 2>&1 | tail -5
cd "$ROOT"

# ── 5. Install ──
echo "[5/5] Installing..."

mkdir -p "$INSTALL_BIN"
cp "$ROOT/target/release/mem"     "$INSTALL_BIN/mem"
cp "$ROOT/target/release/mem-mcp" "$INSTALL_BIN/mem-mcp"
chmod +x "$INSTALL_BIN/mem" "$INSTALL_BIN/mem-mcp"
echo "  -> $INSTALL_BIN/mem"
echo "  -> $INSTALL_BIN/mem-mcp"

BUNDLE_DIR="$ROOT/apps/desktop/src-tauri/target/release/bundle"

if [ "$OS" = "darwin" ]; then
  APP=$(find "$BUNDLE_DIR/macos" -name "*.app" -maxdepth 1 2>/dev/null | head -1)
  if [ -n "$APP" ]; then
    APP_NAME="$(basename "$APP")"
    rm -rf "/Applications/${APP_NAME}"
    cp -R "$APP" /Applications/
    xattr -cr "/Applications/${APP_NAME}" 2>/dev/null || true
    echo "  -> /Applications/${APP_NAME}"
  fi
elif [ "$OS" = "linux" ]; then
  DEB=$(find "$BUNDLE_DIR/deb" -name "*.deb" 2>/dev/null | head -1)
  AI=$(find "$BUNDLE_DIR/appimage" -name "*.AppImage" 2>/dev/null | head -1)
  if [ -n "$DEB" ]; then
    sudo dpkg -i "$DEB"
    echo "  -> installed via dpkg"
  elif [ -n "$AI" ]; then
    cp "$AI" "$INSTALL_BIN/mem-desktop"
    chmod +x "$INSTALL_BIN/mem-desktop"
    DDIR="$HOME/.local/share/applications"
    mkdir -p "$DDIR"
    cat > "$DDIR/mem.desktop" <<DESKTOP
[Desktop Entry]
Name=mem
Comment=Minimal knowledge keeper
Exec=$INSTALL_BIN/mem-desktop
Type=Application
Categories=Utility;
DESKTOP
    echo "  -> $INSTALL_BIN/mem-desktop"
  fi
fi

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_BIN"; then
  echo ""
  echo "  Add to your shell profile:"
  echo "    export PATH=\"$INSTALL_BIN:\$PATH\""
fi

echo ""
echo "  Done. Run 'mem --help' to get started."
echo ""
