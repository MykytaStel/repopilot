#!/usr/bin/env bash
set -euo pipefail

# Quick local view of the broken-code demo: build the scenario in a temp
# repo, run `repopilot review`, then clean up. To record the GIF instead, use
# docs/demos/05-broken-code.tape (VHS), which reuses scripts/demo-broken-code.sh.
#
# Usage: scripts/demo-broken-code-review.sh [path-to-repopilot-binary]

BINARY="${1:-repopilot}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

WORKDIR="$(mktemp -d)"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

"$SCRIPT_DIR/demo-broken-code.sh" "$WORKDIR"
(cd "$WORKDIR" && "$BINARY" review .)
