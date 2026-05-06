#!/usr/bin/env bash
# install.sh — downloads and installs the latest repopilot release binary.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | sh
#
# The binary is placed in ~/.local/bin (created if needed). If you want a system-wide
# install, re-run with sudo and set INSTALL_DIR=/usr/local/bin.

set -euo pipefail

REPO="MykytaStel/repopilot"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BINARY="repopilot"

# ── Detect OS and architecture ────────────────────────────────────────────────

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
      aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
      *)       echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac
    EXT="tar.gz"
    ;;
  Darwin)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-apple-darwin" ;;
      arm64)   TARGET="aarch64-apple-darwin" ;;
      *)       echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac
    EXT="tar.gz"
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    echo "For Windows, download the binary from https://github.com/$REPO/releases" >&2
    exit 1
    ;;
esac

# ── Resolve latest version ────────────────────────────────────────────────────

if command -v curl >/dev/null 2>&1; then
  FETCH="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
  FETCH="wget -qO-"
else
  echo "curl or wget is required" >&2
  exit 1
fi

VERSION=$(
  $FETCH "https://api.github.com/repos/$REPO/releases/latest" |
  grep '"tag_name"' |
  sed 's/.*"tag_name": *"v\([^"]*\)".*/\1/'
)

if [ -z "$VERSION" ]; then
  echo "Could not determine the latest version." >&2
  exit 1
fi

echo "Installing repopilot v$VERSION for $TARGET ..."

# ── Download and verify ───────────────────────────────────────────────────────

ARCHIVE="${BINARY}-v${VERSION}-${TARGET}.${EXT}"
BASE_URL="https://github.com/$REPO/releases/download/v$VERSION"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

$FETCH "$BASE_URL/$ARCHIVE" -o "$TMP_DIR/$ARCHIVE"

if $FETCH "$BASE_URL/$ARCHIVE.sha256" -o "$TMP_DIR/$ARCHIVE.sha256" 2>/dev/null; then
  (cd "$TMP_DIR" && sha256sum -c "$ARCHIVE.sha256" >/dev/null 2>&1) || {
    echo "SHA256 verification failed — aborting." >&2
    exit 1
  }
fi

# ── Extract and install ───────────────────────────────────────────────────────

tar -xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR"

mkdir -p "$INSTALL_DIR"
install -m 755 "$TMP_DIR/$BINARY" "$INSTALL_DIR/$BINARY"

echo "Installed to $INSTALL_DIR/$BINARY"

# Warn if INSTALL_DIR is not in PATH
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo "Note: add $INSTALL_DIR to your PATH to use repopilot directly." ;;
esac
