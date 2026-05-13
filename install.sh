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
ARCHIVE_URL="$BASE_URL/$ARCHIVE"
CHECKSUM="$ARCHIVE.sha256"
CHECKSUM_URL="$BASE_URL/$CHECKSUM"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

if ! download_file "$ARCHIVE_URL" "$TMP_DIR/$ARCHIVE"; then
  echo "Failed to download release archive: $ARCHIVE_URL" >&2
  echo "Installation aborted." >&2
  exit 1
fi

if ! download_file "$CHECKSUM_URL" "$TMP_DIR/$CHECKSUM"; then
  echo "Failed to download required checksum file: $CHECKSUM_URL" >&2
  echo "Installation aborted for safety; cannot verify $ARCHIVE." >&2
  exit 1
fi

verify_checksum() {
  local archive_path="$1"
  local checksum_path="$2"
  local archive_name
  local expected
  local actual

  if ! archive_name="$(basename "$archive_path")"; then
    echo "Could not determine archive name for checksum verification: $archive_path" >&2
    return 1
  fi

  if ! expected="$(awk 'NF { print $1; exit }' "$checksum_path")"; then
    echo "Could not read SHA256 checksum file: $checksum_path" >&2
    return 1
  fi

  if [[ ! "$expected" =~ ^[[:xdigit:]]{64}$ ]]; then
    echo "Invalid SHA256 checksum file: $checksum_path" >&2
    return 1
  fi

  if command -v sha256sum >/dev/null 2>&1; then
    if ! actual="$(sha256sum "$archive_path" | awk '{ print $1 }')"; then
      echo "Could not compute SHA256 for $archive_name with sha256sum." >&2
      return 1
    fi
  elif command -v shasum >/dev/null 2>&1; then
    if ! actual="$(shasum -a 256 "$archive_path" | awk '{ print $1 }')"; then
      echo "Could not compute SHA256 for $archive_name with shasum." >&2
      return 1
    fi
  else
    echo "No SHA256 verification tool found. Install sha256sum or shasum and re-run." >&2
    return 1
  fi

  if [[ "$expected" != "$actual" ]]; then
    echo "SHA256 verification failed for $archive_name." >&2
    echo "Expected: $expected" >&2
    echo "Actual:   $actual" >&2
    return 1
  fi
}

if ! verify_checksum "$TMP_DIR/$ARCHIVE" "$TMP_DIR/$CHECKSUM"; then
  echo "Installation aborted for safety." >&2
  exit 1
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
