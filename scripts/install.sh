#!/usr/bin/env bash
# mem installer
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/denyzhirkov/mem/main/scripts/install.sh | sh
#   curl ... | sh -s -- --cli-only
#   curl ... | sh -s -- --app-only
set -euo pipefail

REPO="denyzhirkov/mem"
VERSION="${MEM_VERSION:-latest}"
CLI_DIR="${MEM_CLI_DIR:-$HOME/.local/bin}"
CLI_ONLY=false
APP_ONLY=false

for arg in "$@"; do
  case "$arg" in
    --cli-only) CLI_ONLY=true ;;
    --app-only) APP_ONLY=true ;;
  esac
done

# Detect platform
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac
PLATFORM="${OS}-${ARCH}"

# Resolve version
if [ "$VERSION" = "latest" ]; then
  VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v?([^"]+)".*/\1/')
  if [ -z "$VERSION" ]; then
    echo "Failed to detect latest version. Set MEM_VERSION=x.y.z manually."
    exit 1
  fi
fi

TAG="v${VERSION}"
BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"

echo ""
echo "  mem installer"
echo "  ─────────────"
echo "  Version:  ${VERSION}"
echo "  Platform: ${PLATFORM}"
echo ""

# ── CLI ──
if [ "$APP_ONLY" = false ]; then
  echo "[1/2] Installing CLI..."

  CLI_BINARY="mem-${PLATFORM}"
  CLI_URL="${BASE_URL}/${CLI_BINARY}"
  TMPFILE="$(mktemp)"

  if curl -fsSL "$CLI_URL" -o "$TMPFILE" 2>/dev/null; then
    chmod +x "$TMPFILE"
    mkdir -p "$CLI_DIR"
    mv "$TMPFILE" "$CLI_DIR/mem"
    echo "  -> $CLI_DIR/mem"
  else
    echo "  CLI binary not found at ${CLI_URL}"
    echo "  Skipping CLI install."
    rm -f "$TMPFILE"
  fi

  # Check PATH
  if ! echo "$PATH" | tr ':' '\n' | grep -qx "$CLI_DIR"; then
    echo ""
    echo "  Note: add to your shell profile:"
    echo "    export PATH=\"$CLI_DIR:\$PATH\""
  fi
fi

# ── Desktop App ──
if [ "$CLI_ONLY" = false ]; then
  echo "[2/2] Installing desktop app..."

  if [ "$OS" = "darwin" ]; then
    DMG_NAME="mem_${VERSION}_${ARCH}.dmg"
    DMG_URL="${BASE_URL}/${DMG_NAME}"
    TMPDIR_DMG="$(mktemp -d)"
    DMG_PATH="${TMPDIR_DMG}/mem.dmg"

    if curl -fsSL "$DMG_URL" -o "$DMG_PATH" 2>/dev/null; then
      # Mount, copy, unmount
      MOUNT_POINT="$(mktemp -d)"
      hdiutil attach "$DMG_PATH" -mountpoint "$MOUNT_POINT" -nobrowse -quiet
      APP_NAME="$(ls "$MOUNT_POINT/" | grep '\.app$' | head -1)"
      if [ -n "$APP_NAME" ]; then
        rm -rf "/Applications/${APP_NAME}"
        cp -R "$MOUNT_POINT/$APP_NAME" /Applications/
        echo "  -> /Applications/${APP_NAME}"
      fi
      hdiutil detach "$MOUNT_POINT" -quiet
      rm -rf "$TMPDIR_DMG" "$MOUNT_POINT"
    else
      echo "  DMG not found at ${DMG_URL}"
      echo "  Skipping desktop install."
    fi

  elif [ "$OS" = "linux" ]; then
    # Try AppImage first
    AI_NAME="mem_${VERSION}_${ARCH}.AppImage"
    AI_URL="${BASE_URL}/${AI_NAME}"
    AI_DEST="$HOME/.local/bin/mem-desktop"

    if curl -fsSL "$AI_URL" -o "$AI_DEST" 2>/dev/null; then
      chmod +x "$AI_DEST"
      echo "  -> $AI_DEST"

      # Create .desktop entry
      DESKTOP_DIR="$HOME/.local/share/applications"
      mkdir -p "$DESKTOP_DIR"
      cat > "$DESKTOP_DIR/mem.desktop" <<DESKTOP
[Desktop Entry]
Name=mem
Comment=Minimal knowledge keeper
Exec=$AI_DEST
Type=Application
Categories=Utility;
DESKTOP
      echo "  -> $DESKTOP_DIR/mem.desktop"
    else
      echo "  AppImage not found. Trying .deb..."
      DEB_NAME="mem_${VERSION}_amd64.deb"
      DEB_URL="${BASE_URL}/${DEB_NAME}"
      TMPFILE="$(mktemp)"
      if curl -fsSL "$DEB_URL" -o "$TMPFILE" 2>/dev/null; then
        sudo dpkg -i "$TMPFILE"
        rm -f "$TMPFILE"
        echo "  -> installed via dpkg"
      else
        echo "  No desktop package found. Skipping."
        rm -f "$TMPFILE"
      fi
    fi
  else
    echo "  Desktop auto-install not supported on $OS."
    echo "  Download manually: https://github.com/${REPO}/releases/tag/${TAG}"
  fi
fi

echo ""
echo "  Done! Run 'mem --help' to get started."
echo ""
