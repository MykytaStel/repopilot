#!/usr/bin/env bash
# install.sh — downloads and installs the latest repopilot release binary.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | bash
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
  fetch_stdout() {
    curl -fsSL "$1"
  }
  download_file() {
    curl -fsSL "$1" -o "$2"
  }
elif command -v wget >/dev/null 2>&1; then
  fetch_stdout() {
    wget -qO- "$1"
  }
  download_file() {
    wget -qO "$2" "$1"
  }
else
  echo "curl or wget is required" >&2
  exit 1
fi

VERSION=$(
  fetch_stdout "https://api.github.com/repos/$REPO/releases/latest" |
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

download_file "$BASE_URL/$ARCHIVE" "$TMP_DIR/$ARCHIVE"

verify_checksum() {
  archive_path="$1"
  checksum_path="$2"

  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$(dirname "$archive_path")" && sha256sum -c "$(basename "$checksum_path")" >/dev/null 2>&1)
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    expected="$(awk '{print $1}' "$checksum_path")"
    actual="$(shasum -a 256 "$archive_path" | awk '{print $1}')"
    [ "$expected" = "$actual" ]
    return
  fi

  echo "No SHA256 tool found; skipping checksum verification." >&2
  return 0
}

if download_file "$BASE_URL/$ARCHIVE.sha256" "$TMP_DIR/$ARCHIVE.sha256" 2>/dev/null; then
  if ! verify_checksum "$TMP_DIR/$ARCHIVE" "$TMP_DIR/$ARCHIVE.sha256"; then
    echo "SHA256 verification failed — aborting." >&2
    exit 1
  fi
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
