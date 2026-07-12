#!/usr/bin/env bash
# shellcheck disable=SC2154

REVIEW_DELTA_FILE="repopilot-review-delta.json"
DELTA_TEMP_DIR=""
TMP_BASE_WORKTREE=""
TMP_HEAD_WORKTREE=""
DELTA_SCAN_ARGS=()

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

delta_scan_args() {
  local output="$1"
  DELTA_SCAN_ARGS=(scan "$PATH_INPUT" --format json --output "$output" --no-progress)
  if [[ -n "$CONFIG" ]]; then DELTA_SCAN_ARGS+=(--config "$CONFIG"); fi
  if [[ -n "$PROFILE" ]]; then DELTA_SCAN_ARGS+=(--profile "$PROFILE"); fi
  if [[ -n "$MIN_SEVERITY" ]]; then DELTA_SCAN_ARGS+=(--min-severity "$MIN_SEVERITY"); fi
  if [[ -n "$MIN_PRIORITY" ]]; then DELTA_SCAN_ARGS+=(--min-priority "$MIN_PRIORITY"); fi
}

build_review_delta() {
  if [[ -z "$BASE" ]]; then
    return 1
  fi

  local workspace base_scan head_scan base_status head_status head_revision
  workspace="$PWD"
  DELTA_TEMP_DIR="$(mktemp -d "${RUNNER_TEMP:-/tmp}/repopilot-delta.XXXXXX")"
  TMP_BASE_WORKTREE="$DELTA_TEMP_DIR/base-worktree"
  TMP_HEAD_WORKTREE="$DELTA_TEMP_DIR/head-worktree"
  base_scan="$DELTA_TEMP_DIR/base.json"
  head_scan="$DELTA_TEMP_DIR/head.json"
  head_revision="${HEAD:-$(git rev-parse HEAD)}"

  git worktree add --detach --quiet "$TMP_BASE_WORKTREE" "$BASE"
  delta_scan_args "$base_scan"
  set +e
  (cd "$TMP_BASE_WORKTREE" && repopilot "${DELTA_SCAN_ARGS[@]}")
  base_status=$?
  set -e
  git worktree remove --force "$TMP_BASE_WORKTREE" >/dev/null
  TMP_BASE_WORKTREE=""
  if [[ "$base_status" -ne 0 ]]; then
    echo "::warning::RepoPilot could not scan base revision $BASE; delta artifact was not produced."
    return 1
  fi

  git worktree add --detach --quiet "$TMP_HEAD_WORKTREE" "$head_revision"
  delta_scan_args "$head_scan"
  set +e
  (cd "$TMP_HEAD_WORKTREE" && repopilot "${DELTA_SCAN_ARGS[@]}")
  head_status=$?
  set -e
  git worktree remove --force "$TMP_HEAD_WORKTREE" >/dev/null
  TMP_HEAD_WORKTREE=""
  if [[ "$head_status" -ne 0 ]]; then
    echo "::warning::RepoPilot could not scan head revision $head_revision; delta artifact was not produced."
    return 1
  fi

  python3 "$ACTION_ROOT/scripts/review_delta.py" \
    --base "$base_scan" \
    --head "$head_scan" \
    --output "$workspace/$REVIEW_DELTA_FILE" \
    --base-revision "$BASE" \
    --head-revision "$head_revision"
}

write_review_summary() {
  local review_json="$1"
  REVIEW_SUMMARY_FILE="repopilot-review-summary.md"
  {
    echo "## RepoPilot Review"
    echo
    if [[ -f "$REVIEW_DELTA_FILE" ]]; then
      echo "- **New findings:** $(jq -r '.summary.new_findings' "$REVIEW_DELTA_FILE")"
      echo "- **Changed findings:** $(jq -r '.summary.changed_findings' "$REVIEW_DELTA_FILE")"
      echo "- **Resolved findings:** $(jq -r '.summary.resolved_findings' "$REVIEW_DELTA_FILE")"
    else
      echo "- **In-diff findings:** $(jq -r '.review.in_diff_findings' "$review_json")"
    fi
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
        [.new_findings[], .changed_findings[]][0:20][]
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
       + [$delta[0].new_findings[], $delta[0].changed_findings[] | ["warning", (.evidence[0].path // ""), (.evidence[0].line_start // 1), .title]])[0:20][]
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
      echo "new_findings_count=$(jq -r '.summary.new_findings' "$REVIEW_DELTA_FILE")"
      echo "changed_findings_count=$(jq -r '.summary.changed_findings' "$REVIEW_DELTA_FILE")"
      echo "resolved_findings_count=$(jq -r '.summary.resolved_findings' "$REVIEW_DELTA_FILE")"
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
