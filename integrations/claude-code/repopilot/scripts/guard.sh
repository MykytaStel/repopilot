#!/usr/bin/env sh
# Stop: review everything this session changed. Block the stop once when
# RepoPilot finds a definitely-sensitive signal (a committed focused test, a
# removed auth check, request input reaching a shell, ...) or any test-integrity
# signal (a skipped or removed test, lost assertions, a new suppression), and
# hand Claude the list. Exit 2 feeds stderr back to Claude; exit 0 lets it stop.
input=$(cat)
case "$input" in
  *'"stop_hook_active":true'* | *'"stop_hook_active": true'*) exit 0 ;; # one re-prompt only
esac
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || exit 0
command -v repopilot >/dev/null 2>&1 || {
  echo "RepoPilot: session review unavailable; install the repopilot CLI (0.24 or newer) on PATH." >&2
  exit 0
}
version=$(repopilot --version 2>/dev/null) || version=""
if ! printf '%s\n' "$version" | awk '
  $1 == "repopilot" { split($2, v, "."); ready = v[1] ~ /^[0-9]+$/ && v[2] ~ /^[0-9]+$/ && (v[1]+0 > 0 || v[2]+0 >= 24) }
  END { exit !ready }
'; then
  echo "RepoPilot: session review unavailable; repopilot on PATH must be 0.24 or newer. Check repopilot --version." >&2
  exit 0
fi
root=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
[ -f "$root/.repopilot/snapshot.json" ] || {
  echo "RepoPilot: session review unavailable; snapshot is missing. Start a new agent session." >&2
  exit 0
}
[ -w "$root/.repopilot" ] && [ -w "$root/.repopilot/snapshot.json" ] || {
  echo "RepoPilot: session review unavailable; restore .repopilot write permission, then start a new session." >&2
  exit 0
}

out=$(repopilot review "$root" --since-snapshot 2>&1) || {
  echo "RepoPilot: session review unavailable; review failed. Run repopilot review --since-snapshot to inspect the error." >&2
  exit 0
}

flagged=$(printf '%s\n' "$out" | awk '
  /^  Definitely sensitive:/ { tier = "definitely"; next }
  /^  Maybe sensitive:/ { tier = "maybe"; next }
  /^  [A-Z]/ || /^$/ || /^[^ ]/ { tier = "" }
  tier == "definitely" && /⚑/ { print; next }
  tier == "maybe" && /⚑ (test skipped|test removed|assertions removed|suppression added|check gate relaxed|RepoPilot suppression added) — / { print }
')
[ -z "$flagged" ] && exit 0
{
  echo "RepoPilot reviewed this session and found changes that need attention before you finish:"
  printf '%s\n' "$flagged"
  echo "Restore each weakened check or fix the flagged code, or explain in your reply why the change is intended."
  echo "Re-check with: repopilot review --since-snapshot"
} >&2
exit 2
