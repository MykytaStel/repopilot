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
command -v repopilot >/dev/null 2>&1 || exit 0
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || exit 0
root=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
[ -f "$root/.repopilot/snapshot.json" ] || exit 0

out=$(repopilot review "$root" --since-snapshot 2>&1) || exit 0

flagged=$(printf '%s\n' "$out" | awk '
  /^  Definitely sensitive:/ { tier = "definitely"; next }
  /^  Maybe sensitive:/ { tier = "maybe"; next }
  /^  [A-Z]/ || /^$/ || /^[^ ]/ { tier = "" }
  tier == "definitely" && /⚑/ { print; next }
  tier == "maybe" && /⚑ (test skipped|test removed|assertions removed|suppression added|check gate relaxed) — / { print }
')
[ -z "$flagged" ] && exit 0
{
  echo "RepoPilot reviewed this session and found changes that need attention before you finish:"
  printf '%s\n' "$flagged"
  echo "Restore each weakened check or fix the flagged code, or explain in your reply why the change is intended."
  echo "Re-check with: repopilot review --since-snapshot"
} >&2
exit 2
