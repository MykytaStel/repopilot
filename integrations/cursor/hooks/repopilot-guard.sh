#!/usr/bin/env sh
# Cursor stop: review everything this agent session changed. When the session
# weakened a check or a safeguard (a focused, skipped, or removed test, lost
# assertions, a new suppression, a relaxed CI gate, a removed auth check,
# request input newly reaching SQL or a shell), send the agent the list as one
# follow-up message. Other sensitive signals, such as a touched workflow or a
# dependency bump, are review context. Each signal is raised once per session.
# Prints `{}` when there is nothing to report, and never blocks on its own failure.
input=$(cat)
quiet() { echo '{}'; exit 0; }
unavailable() { echo "RepoPilot: session review unavailable; $1" >&2; quiet; }
case "$input" in
  *'"status":"completed"'* | *'"status": "completed"'*) ;;
  *) quiet ;;
esac
case "$input" in
  *'"loop_count":0'* | *'"loop_count": 0'*) ;;
  *) quiet ;; # one follow-up per session stop
esac
root=$(git rev-parse --show-toplevel 2>/dev/null) || quiet
command -v repopilot >/dev/null 2>&1 || unavailable "install the repopilot CLI (0.24 or newer) on PATH."
version=$(repopilot --version 2>/dev/null) || version=""
if ! printf '%s\n' "$version" | awk '
  $1 == "repopilot" { split($2, v, "."); ready = v[1] ~ /^[0-9]+$/ && v[2] ~ /^[0-9]+$/ && (v[1]+0 > 0 || v[2]+0 >= 24) }
  END { exit !ready }
'; then
  unavailable "repopilot on PATH must be 0.24 or newer. Check repopilot --version."
fi
[ -f "$root/.repopilot/snapshot.json" ] || unavailable "snapshot is missing. Start a new agent session."
[ -w "$root/.repopilot" ] && [ -w "$root/.repopilot/snapshot.json" ] ||
  unavailable "restore .repopilot write permission, then start a new session."

# --- shared: weakened-signal selection (keep identical in Claude Code and Cursor guards) ---
# JSON lists every signal; the console preview caps its list. A configured
# review gate must not turn a finding into a "review failed" exit.
report=$(repopilot review "$root" --since-snapshot --fail-on-review none --format json 2>/dev/null) ||
  unavailable "review failed. Run repopilot review --since-snapshot to inspect the error."
# One tab-separated row per unsuppressed signal that weakened a check or a
# safeguard: kind, path, line, headline, detail.
flagged=$(printf '%s\n' "$report" | awk '
  /^  "tiered_signals": \{/ { inside = 1; next }
  inside && /^  [}"]/ { inside = 0 }
  !inside { next }
  /^    "(definitely|maybe)": \[/ { listed = 1; next }
  /^    "[a-z]+": \[/ { listed = 0; next }
  !listed { next }
  /^      \{/ { kind = path = line = headline = detail = ""; suppressed = "false"; next }
  /^        "[a-z_]+": / {
    name = $0; sub(/^        "/, "", name); value = name
    sub(/".*/, "", name); sub(/^[a-z_]+": /, "", value); sub(/,$/, "", value)
    if (value == "null") value = ""
    if (value ~ /^".*"$/) { value = substr(value, 2, length(value) - 2); gsub(/\\"/, "\"", value) }
    if (name == "kind") kind = value
    else if (name == "path") path = value
    else if (name == "line") line = value
    else if (name == "headline") headline = value
    else if (name == "detail") detail = value
    else if (name == "suppressed") suppressed = value
    next
  }
  /^      \}/ {
    weakened = kind ~ /^integrity\./ || kind == "behavioral.auth-check-removed" ||
      kind == "behavioral.test-deleted-or-emptied" || kind == "taint.sql" || kind == "taint.exec"
    if (weakened && suppressed == "false") print kind "\t" path "\t" line "\t" headline "\t" detail
  }
')
# Raise each signal once per snapshot. A signal the agent already answered,
# by restoring it or explaining it, does not stop every later turn again. A
# new weakening does, and so does one that returns after a restore.
session=$(awk '{ printf "%s", $0 }' "$root/.repopilot/snapshot.json")
raised=$(git -C "$root" rev-parse --git-path repopilot-raised-signals 2>/dev/null) || raised=""
case "$raised" in /* | '') ;; *) raised="$root/$raised" ;; esac
new=$(printf '%s\n' "$flagged" | awk -v raised="$raised" -v session="$session" '
  BEGIN {
    FS = "\t"
    if (raised != "" && (getline first < raised) > 0 && first == session)
      while ((getline key < raised) > 0) seen[key]++
  }
  NF == 0 { next }
  {
    key = $1 "\t" $2 "\t" $5
    if (++now[key] <= seen[key]) next
    location = $2; if ($3 != "") location = location ":" $3
    detail = ($5 == "") ? "" : ("  " $5)
    print "  ⚑ " $4 " — " location detail
  }
')
if [ -n "$raised" ]; then
  ( { printf '%s\n' "$session"; printf '%s\n' "$flagged" | awk 'BEGIN { FS = "\t" } NF { print $1 "\t" $2 "\t" $5 }'; } >"$raised" ) 2>/dev/null
fi
# --- end shared ---
[ -z "$new" ] && quiet

# JSON string: drop control characters, escape backslashes and quotes, join lines with \n.
message=$({
  echo "RepoPilot reviewed this session and found weakened checks or safeguards:"
  printf '%s\n' "$new"
  echo "Restore or fix each one, or explain in your reply why it is intended. RepoPilot raises each signal once per session."
  echo "Re-check with: repopilot review --since-snapshot"
} | tr '\t' ' ' | tr -d '\000-\010\013-\037' | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' |
  awk 'NR > 1 { printf "%s", "\\n" } { printf "%s", $0 }')
printf '{"followup_message":"%s"}\n' "$message"
