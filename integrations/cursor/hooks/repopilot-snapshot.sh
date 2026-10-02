#!/usr/bin/env sh
# Cursor sessionStart: mark the repository state before the agent changes
# anything, so the stop hook reviews only this session. Never blocks.
cat >/dev/null
quiet() { echo '{}'; exit 0; }
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || quiet
root=$(git rev-parse --show-toplevel 2>/dev/null) || quiet
if [ -d "$root/.repopilot" ] && [ ! -w "$root/.repopilot" ]; then
  echo "RepoPilot: session review unavailable; restore .repopilot write permission, then start a new session. Prior metadata could not be invalidated." >&2
  quiet
fi
# Delete the old marker, or empty it if an ACL denies deletion but permits writes.
invalidate_snapshot() {
  rm -f "$root/.repopilot/snapshot.json" 2>/dev/null && return 0
  ( : > "$root/.repopilot/snapshot.json" ) 2>/dev/null && return 0
  return 1
}
# A new session must not inherit the previous session's baseline if setup fails.
if ! invalidate_snapshot; then
  echo "RepoPilot: session review unavailable; restore snapshot write permission, then start a new session. Prior metadata could not be invalidated." >&2
  quiet
fi
command -v repopilot >/dev/null 2>&1 || {
  echo "RepoPilot: session review unavailable; install the repopilot CLI (0.24 or newer) on PATH." >&2
  quiet
}
version=$(repopilot --version 2>/dev/null) || version=""
if ! printf '%s\n' "$version" | awk '
  $1 == "repopilot" { split($2, v, "."); ready = v[1] ~ /^[0-9]+$/ && v[2] ~ /^[0-9]+$/ && (v[1]+0 > 0 || v[2]+0 >= 24) }
  END { exit !ready }
'; then
  echo "RepoPilot: session review unavailable; repopilot on PATH must be 0.24 or newer. Check repopilot --version." >&2
  quiet
fi
if ! repopilot snapshot >/dev/null 2>&1; then
  if ! invalidate_snapshot; then
    echo "RepoPilot: session review unavailable; restore snapshot write permission, then start a new session." >&2
    quiet
  fi
  echo "RepoPilot: session review unavailable; snapshot failed. Run repopilot snapshot, then retry." >&2
fi
echo '{}'
