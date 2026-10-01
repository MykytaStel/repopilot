#!/usr/bin/env sh
# Cursor sessionStart: mark the repository state before the agent changes
# anything, so the stop hook reviews only this session. Never blocks.
cat >/dev/null
if command -v repopilot >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  repopilot snapshot >/dev/null 2>&1 || true
fi
echo '{}'
