#!/usr/bin/env bash
# shellcheck disable=SC2154

REVIEW_DELTA_FILE="repopilot-review-delta.json"
DELTA_TEMP_DIR=""
TMP_BASE_WORKTREE=""
TMP_HEAD_WORKTREE=""

cleanup_review_temp() {
  if [[ -n "$TMP_BASE_WORKTREE" && -d "$TMP_BASE_WORKTREE" ]]; then
    git worktree remove --force "$TMP_BASE_WORKTREE" >/dev/null 2>&1 || true
  fi
  if [[ -n "$TMP_HEAD_WORKTREE" && -d "$TMP_HEAD_WORKTREE" ]]; then
    git worktree remove --force "$TMP_HEAD_WORKTREE" >/dev/null 2>&1 || true
  fi
  if [[ -n "$DELTA_TEMP_DIR" && -d "$DELTA_TEMP_DIR" ]]; then
    rm -rf "$DELTA_TEMP_DIR"
  fi
}

# Builds the base/head finding delta through RepoPilot's own baseline engine:
# `baseline create` snapshots the base revision, then `scan --baseline`
# against that snapshot on the head revision reports new/existing/resolved
# directly. Both calls share the same visibility filter (see
# `baseline create --help`) so a finding a filter hides on one side never
# misreports as spuriously new or resolved on the other.
build_review_delta() {
  if [[ -z "$BASE" ]]; then
    return 1
  fi

  local workspace base_baseline head_scan base_status head_status head_revision
  workspace="$PWD"
  DELTA_TEMP_DIR="$(mktemp -d "${RUNNER_TEMP:-/tmp}/repopilot-delta.XXXXXX")"
  TMP_BASE_WORKTREE="$DELTA_TEMP_DIR/base-worktree"
  TMP_HEAD_WORKTREE="$DELTA_TEMP_DIR/head-worktree"
  base_baseline="$DELTA_TEMP_DIR/base-baseline.json"
  head_scan="$DELTA_TEMP_DIR/head.json"
  head_revision="${HEAD:-$(git rev-parse HEAD)}"

  local filter_args=()
  if [[ -n "$PROFILE" ]]; then filter_args+=(--profile "$PROFILE"); fi
  if [[ -n "$MIN_SEVERITY" ]]; then filter_args+=(--min-severity "$MIN_SEVERITY"); fi
  if [[ -n "$MIN_PRIORITY" ]]; then filter_args+=(--min-priority "$MIN_PRIORITY"); fi

  git worktree add --detach --quiet "$TMP_BASE_WORKTREE" "$BASE"
  set +e
  # bash 3.2 (macOS's system default) treats "${arr[@]}" on an empty array as
  # an unbound-variable error under `set -u`; the `+` form only expands when
  # the array is set, which an empty array still satisfies.
  (cd "$TMP_BASE_WORKTREE" && repopilot baseline create "$PATH_INPUT" \
    --output "$base_baseline" --force "${filter_args[@]+"${filter_args[@]}"}")
  base_status=$?
  set -e
  git worktree remove --force "$TMP_BASE_WORKTREE" >/dev/null
  TMP_BASE_WORKTREE=""
  if [[ "$base_status" -ne 0 ]]; then
    echo "::warning::RepoPilot could not baseline base revision $BASE; delta artifact was not produced."
    return 1
  fi

  git worktree add --detach --quiet "$TMP_HEAD_WORKTREE" "$head_revision"
  set +e
  (cd "$TMP_HEAD_WORKTREE" && repopilot scan "$PATH_INPUT" --baseline "$base_baseline" \
    --format json --output "$head_scan" --no-progress "${filter_args[@]+"${filter_args[@]}"}")
  head_status=$?
  set -e
  git worktree remove --force "$TMP_HEAD_WORKTREE" >/dev/null
  TMP_HEAD_WORKTREE=""
  if [[ "$head_status" -ne 0 ]]; then
    echo "::warning::RepoPilot could not scan head revision $head_revision against the base baseline; delta artifact was not produced."
    return 1
  fi

  cp "$head_scan" "$workspace/$REVIEW_DELTA_FILE"
}

write_review_summary() {
  local review_json="$1"
  REVIEW_SUMMARY_FILE="repopilot-review-summary.md"
  {
    echo "## RepoPilot Review"
    echo
    if [[ -f "$REVIEW_DELTA_FILE" ]]; then
      echo "- **New findings:** $(jq -r '.baseline.new_findings' "$REVIEW_DELTA_FILE")"
      echo "- **Resolved findings:** $(jq -r '.baseline.resolved_findings' "$REVIEW_DELTA_FILE")"
    else
      echo "- **In-diff findings:** $(jq -r '.review.in_diff_findings' "$review_json")"
    fi
    echo "- **Merge readiness:** $(jq -r '.merge_readiness.verdict // "unavailable"' "$review_json")"
    echo "- **Definitely-sensitive signals:** $(jq -r '.review.tiered_signals.definitely' "$review_json")"
    echo "- **Maybe-sensitive signals:** $(jq -r '.review.tiered_signals.maybe' "$review_json")"
    echo "- **Review gate:** $(jq -r '.review_gate.status // "not-configured"' "$review_json")"
    echo
    jq -r '
      [.tiered_signals.definitely[], .tiered_signals.maybe[]]
      | map(select(.suppressed == false))[0:20][]
      | "- **\(.headline)** — `\(.path)\(if .line_start then ":\(.line_start)" else "" end)`\(if .detail then ": \(.detail)" else "" end)"
    ' "$review_json"
    if [[ -f "$REVIEW_DELTA_FILE" ]]; then
      jq -r '
        [.findings[] | select(.baseline_status == "new")][0:20][]
        | "- **\(.title)** — `\(.evidence[0].path // "."):\(.evidence[0].line_start // 1)`"
      ' "$REVIEW_DELTA_FILE"
    fi
  } > "$REVIEW_SUMMARY_FILE"
}

annotation_rows() {
  local review_json="$1"
  if [[ -f "$REVIEW_DELTA_FILE" ]]; then
    jq -r --slurpfile delta "$REVIEW_DELTA_FILE" '
      ([.tiered_signals.definitely[] | select(.suppressed == false) | ["warning", .path, (.line_start // 1), (.headline + (if .detail then ": " + .detail else "" end))]]
       + [.tiered_signals.maybe[] | select(.suppressed == false) | ["notice", .path, (.line_start // 1), (.headline + (if .detail then ": " + .detail else "" end))]]
       + [$delta[0].findings[] | select(.baseline_status == "new") | ["warning", (.evidence[0].path // ""), (.evidence[0].line_start // 1), .title]])[0:20][]
      | @tsv
    ' "$review_json"
  else
    jq -r '
      ([.tiered_signals.definitely[] | select(.suppressed == false) | ["warning", .path, (.line_start // 1), (.headline + (if .detail then ": " + .detail else "" end))]]
       + [.tiered_signals.maybe[] | select(.suppressed == false) | ["notice", .path, (.line_start // 1), (.headline + (if .detail then ": " + .detail else "" end))]]
       + [.findings[] | select(.in_diff == true) | ["warning", (.evidence[0].path // ""), (.evidence[0].line_start // 1), .title]])[0:20][]
      | @tsv
    ' "$review_json"
  fi
}

emit_review_annotations() {
  local review_json="$1" level path line message
  while IFS=$'\t' read -r level path line message; do
    [[ -n "$path" ]] || continue
    message="${message//%/%25}"
    message="${message//$'\r'/'%0D'}"
    message="${message//$'\n'/'%0A'}"
    echo "::${level} file=${path},line=${line:-1}::${message}"
  done < <(annotation_rows "$review_json")
}

write_review_outputs() {
  local review_json="$1"
  [[ -n "${GITHUB_OUTPUT:-}" ]] || return 0
  {
    echo "review_json_file=$review_json"
    echo "review_sarif_file=$SARIF_OUTPUT"
    echo "sarif_file=$SARIF_OUTPUT"
    echo "summary_file=$REVIEW_SUMMARY_FILE"
    if [[ -f "$REVIEW_DELTA_FILE" ]]; then
      echo "delta_json_file=$REVIEW_DELTA_FILE"
      echo "new_findings_count=$(jq -r '.baseline.new_findings' "$REVIEW_DELTA_FILE")"
      # No "changed" bucket exists in the baseline model: a finding whose
      # snippet/title survive a line shift reads as Existing there, by
      # design, so there is nothing left this count would ever report.
      # Declared for output-name compatibility only.
      echo "changed_findings_count=0"
      echo "resolved_findings_count=$(jq -r '.baseline.resolved_findings' "$REVIEW_DELTA_FILE")"
    fi
    echo "findings_count=$(jq -r '.review.in_diff_findings' "$review_json")"
    echo "signals_count=$(jq -r '.review.tiered_signals.total' "$review_json")"
    echo "gate_result=$(jq -r '.review_gate.status // "not-configured"' "$review_json")"
  } >> "$GITHUB_OUTPUT"
}

finalize_review_artifacts() {
  local review_json="$1"
  if ! build_review_delta; then
    rm -f "$REVIEW_DELTA_FILE"
  fi
  write_review_summary "$review_json"
  emit_review_annotations "$review_json"
  write_review_outputs "$review_json"
}
