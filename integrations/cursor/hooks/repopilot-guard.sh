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
root=$(git rev-parse --show-toplevel 2>/dev/null) || quiet
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
[ -f "$root/.repopilot/snapshot.json" ] || {
  echo "RepoPilot: session review unavailable; snapshot is missing. Start a new agent session." >&2
  quiet
}
[ -w "$root/.repopilot" ] && [ -w "$root/.repopilot/snapshot.json" ] || {
  echo "RepoPilot: session review unavailable; restore .repopilot write permission, then start a new session." >&2
  quiet
}

out=$(repopilot review "$root" --since-snapshot 2>&1) || {
  echo "RepoPilot: session review unavailable; review failed. Run repopilot review --since-snapshot to inspect the error." >&2
  quiet
}

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
