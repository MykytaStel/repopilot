#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?version is required}"
CACHE_ROOT="${HOME}/.cache/repopilot-action/${VERSION}"
mkdir -p "$CACHE_ROOT"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) TARGET="aarch64-apple-darwin"; EXT="tar.gz"; BINARY="repopilot" ;;
  Darwin-x86_64) TARGET="x86_64-apple-darwin"; EXT="tar.gz"; BINARY="repopilot" ;;
  Linux-aarch64|Linux-arm64) TARGET="aarch64-unknown-linux-gnu"; EXT="tar.gz"; BINARY="repopilot" ;;
  Linux-x86_64) TARGET="x86_64-unknown-linux-gnu"; EXT="tar.gz"; BINARY="repopilot" ;;
  MINGW*-x86_64|MSYS*-x86_64) TARGET="x86_64-pc-windows-msvc"; EXT="zip"; BINARY="repopilot.exe" ;;
  *)
    echo "::error::Unsupported runner platform $(uname -s)-$(uname -m)"
    exit 3
    ;;
esac

DEST="${CACHE_ROOT}/${BINARY}"
if [[ ! -x "$DEST" && ! -f "$DEST" ]]; then
  ASSET="repopilot-v${VERSION}-${TARGET}.${EXT}"
  URL="https://github.com/MykytaStel/repopilot/releases/download/v${VERSION}/${ASSET}"
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT
  curl --fail --location --silent --show-error "$URL" --output "$TMP/$ASSET"
  curl --fail --location --silent --show-error "$URL.sha256" --output "$TMP/$ASSET.sha256"
  (
    cd "$TMP"
    if command -v sha256sum >/dev/null 2>&1; then
      sha256sum --check "$ASSET.sha256"
    else
      shasum -a 256 --check "$ASSET.sha256"
    fi
  )
  if [[ "$EXT" == "zip" ]]; then
    unzip -q "$TMP/$ASSET" -d "$TMP/extract"
  else
    mkdir -p "$TMP/extract"
    tar -xzf "$TMP/$ASSET" -C "$TMP/extract"
  fi
  FOUND="$(find "$TMP/extract" -type f -name "$BINARY" -print -quit)"
  [[ -n "$FOUND" ]] || { echo "::error::$ASSET does not contain $BINARY"; exit 3; }
  cp "$FOUND" "$DEST"
  chmod +x "$DEST" || true
fi

echo "$CACHE_ROOT" >> "$GITHUB_PATH"
echo "binary=$DEST" >> "$GITHUB_OUTPUT"
