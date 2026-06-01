#!/usr/bin/env bash
set -euo pipefail

# Quick local view of the boundary-review demo: build the scenario in a temp
# repo, run `repopilot review`, then clean up. To record the GIF instead, use
# docs/demos/02-review.tape (VHS), which reuses scripts/demo-setup.sh.
#
# Usage: scripts/demo-boundary-review.sh [path-to-repopilot-binary]

BINARY="${1:-repopilot}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

WORKDIR="$(mktemp -d)"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

"$SCRIPT_DIR/demo-setup.sh" "$WORKDIR"
(cd "$WORKDIR" && "$BINARY" review .)
