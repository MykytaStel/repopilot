#!/usr/bin/env sh
# Cursor stop: review everything this agent session changed. When RepoPilot
# finds a definitely-sensitive signal (a committed focused test, a removed auth
# check, request input reaching a shell, ...) or any test-integrity signal (a
# skipped or removed test, lost assertions, a new suppression, a relaxed CI
# gate), send the agent the list as one follow-up message. Prints `{}` when
# there is nothing to report, and never blocks on its own failure.
input=$(cat)
quiet() { echo '{}'; exit 0; }
case "$input" in
  *'"status":"completed"'* | *'"status": "completed"'*) ;;
  *) quiet ;;
esac
case "$input" in
  *'"loop_count":0'* | *'"loop_count": 0'*) ;;
  *) quiet ;; # one follow-up per session stop
esac
command -v repopilot >/dev/null 2>&1 || quiet
root=$(git rev-parse --show-toplevel 2>/dev/null) || quiet
[ -f "$root/.repopilot/snapshot.json" ] || quiet

out=$(repopilot review "$root" --since-snapshot 2>&1) || quiet

flagged=$(printf '%s\n' "$out" | awk '
  /^  Definitely sensitive:/ { tier = "definitely"; next }
  /^  Maybe sensitive:/ { tier = "maybe"; next }
  /^  [A-Z]/ || /^$/ || /^[^ ]/ { tier = "" }
  tier == "definitely" && /⚑/ { print; next }
  tier == "maybe" && /⚑ (test skipped|test removed|assertions removed|suppression added|check gate relaxed|RepoPilot suppression added) — / { print }
')
[ -z "$flagged" ] && quiet

# JSON string: drop control characters, escape backslashes and quotes, join lines with \n.
message=$({
  echo "RepoPilot reviewed this session and found changes that need attention before you finish:"
  printf '%s\n' "$flagged"
  echo "Restore each weakened check or fix the flagged code, or explain in your reply why the change is intended."
  echo "Re-check with: repopilot review --since-snapshot"
} | tr '\t' ' ' | tr -d '\000-\010\013-\037' | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' |
  awk 'NR > 1 { printf "%s", "\\n" } { printf "%s", $0 }')
printf '{"followup_message":"%s"}\n' "$message"
