#!/usr/bin/env sh
# SessionStart: mark the repository state before Claude changes anything, so
# the Stop hook reviews only this session. Never blocks the session.
input=$(cat)
case "$input" in
  *'"source":"resume"'* | *'"source": "resume"'* | *'"source":"compact"'* | *'"source": "compact"'*)
    exit 0 ;; # same session continues; keep its snapshot
esac
command -v repopilot >/dev/null 2>&1 || {
  echo "RepoPilot plugin: the repopilot CLI is not installed (npm install -g repopilot or cargo install repopilot); session review is off." >&2
  exit 0
}
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || exit 0
repopilot snapshot >/dev/null 2>&1 || true
exit 0
