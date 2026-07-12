#!/usr/bin/env bash
set -euo pipefail

COMMAND="${INPUT_COMMAND:-scan}"
FORMAT="${INPUT_FORMAT:-auto}"
PATH_INPUT="${INPUT_PATH:-.}"
CONFIG="${INPUT_CONFIG:-}"
BASELINE="${INPUT_BASELINE:-}"
FAIL_ON="${INPUT_FAIL_ON:-}"
FAIL_ON_PRIORITY="${INPUT_FAIL_ON_PRIORITY:-}"
FAIL_ON_REVIEW="${INPUT_FAIL_ON_REVIEW:-none}"
SCOPE="${INPUT_SCOPE:-}"
PROFILE="${INPUT_PROFILE:-}"
MIN_SEVERITY="${INPUT_MIN_SEVERITY:-}"
MIN_PRIORITY="${INPUT_MIN_PRIORITY:-}"
RULE_INPUT="${INPUT_RULE:-}"
TIMING="${INPUT_TIMING:-false}"
BASE="${INPUT_BASE:-}"
HEAD="${INPUT_HEAD:-}"
FOCUS="${INPUT_FOCUS:-}"
BUDGET="${INPUT_BUDGET:-}"
OUTPUT="${INPUT_OUTPUT:-}"
RECEIPT="${INPUT_RECEIPT:-}"
ARGS="${INPUT_ARGS:-}"
SARIF_OUTPUT=""
ACTION_ROOT="${GITHUB_ACTION_PATH:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
# shellcheck source=scripts/repopilot-action-review.sh
source "$ACTION_ROOT/scripts/repopilot-action-review.sh"

RUN_ARGS=()
case "$COMMAND" in
  scan) RUN_ARGS=(scan "$PATH_INPUT") ;;
  review) RUN_ARGS=(review "$PATH_INPUT") ;;
  ai-context) RUN_ARGS=(ai context "$PATH_INPUT") ;;
  *)
    echo "::error::Unsupported command '$COMMAND'. Expected scan, review, or ai-context."
    exit 2
    ;;
esac

IS_AI=false
case "$COMMAND" in
  ai-context) IS_AI=true ;;
esac

if [[ "$FORMAT" == "auto" ]]; then
  if [[ "$COMMAND" == "scan" ]]; then
    FORMAT="sarif"
  elif [[ "$COMMAND" == "review" ]]; then
    FORMAT="json"
  else
    FORMAT="markdown"
  fi
fi

if [[ "$FORMAT" == "sarif" && "$COMMAND" != "scan" ]]; then
  echo "::error::SARIF output and upload are only supported by 'scan'. Use format=markdown for '$COMMAND'."
  exit 2
fi
if [[ -n "$RECEIPT" && "$COMMAND" != "scan" ]]; then
  echo "::error::Receipt output is only supported by 'scan'. Remove receipt or use command=scan."
  exit 2
fi
if [[ -n "$FAIL_ON_PRIORITY" && "$COMMAND" != "scan" && "$COMMAND" != "review" ]]; then
  echo "::error::fail-on-priority is only supported by 'scan' and 'review'."
  exit 2
fi
if [[ -n "$MIN_PRIORITY" && "$COMMAND" != "scan" && "$COMMAND" != "review" ]]; then
  echo "::error::min-priority is only supported by 'scan' and 'review'."
  exit 2
fi
if [[ -n "$RULE_INPUT" && "$COMMAND" != "scan" ]]; then
  echo "::error::rule filtering is only supported by 'scan'."
  exit 2
fi
if [[ "$TIMING" == "true" && "$COMMAND" != "scan" ]]; then
  echo "::error::timing is only supported by 'scan'."
  exit 2
fi

if [[ -n "$CONFIG" ]]; then RUN_ARGS+=(--config "$CONFIG"); fi
if [[ "$COMMAND" == "scan" || "$COMMAND" == "review" ]]; then
  if [[ -n "$BASELINE" ]]; then RUN_ARGS+=(--baseline "$BASELINE"); fi
  if [[ -n "$FAIL_ON" ]]; then RUN_ARGS+=(--fail-on "$FAIL_ON"); fi
  if [[ -n "$FAIL_ON_PRIORITY" ]]; then RUN_ARGS+=(--fail-on-priority "$FAIL_ON_PRIORITY"); fi
  if [[ -n "$MIN_SEVERITY" ]]; then RUN_ARGS+=(--min-severity "$MIN_SEVERITY"); fi
  if [[ -n "$MIN_PRIORITY" ]]; then RUN_ARGS+=(--min-priority "$MIN_PRIORITY"); fi
fi
if [[ "$COMMAND" == "review" ]]; then
  if [[ -n "$FAIL_ON_REVIEW" ]]; then RUN_ARGS+=(--fail-on-review "$FAIL_ON_REVIEW"); fi
  if [[ -n "$SCOPE" ]]; then RUN_ARGS+=(--scope "$SCOPE"); fi
  if [[ -n "$PROFILE" ]]; then RUN_ARGS+=(--profile "$PROFILE"); fi
fi
if [[ "$COMMAND" == "scan" && -n "$RULE_INPUT" ]]; then
  read -r -a RULES <<< "$RULE_INPUT"
  for RULE in "${RULES[@]}"; do
    RUN_ARGS+=(--rule "$RULE")
  done
fi
if [[ "$COMMAND" == "scan" && "$TIMING" == "true" ]]; then RUN_ARGS+=(--timing); fi
if [[ "$COMMAND" == "review" && -n "${GITHUB_EVENT_PATH:-}" && -f "$GITHUB_EVENT_PATH" ]]; then
  if [[ -z "$BASE" ]]; then BASE="$(jq -r '.pull_request.base.sha // empty' "$GITHUB_EVENT_PATH")"; fi
  if [[ -z "$HEAD" ]]; then HEAD="$(jq -r '.pull_request.head.sha // empty' "$GITHUB_EVENT_PATH")"; fi
fi
if [[ "$COMMAND" == "review" && -n "$BASE" ]]; then
  git cat-file -e "${BASE}^{commit}" 2>/dev/null || git fetch --no-tags --depth=1 origin "$BASE"
  RUN_ARGS+=(--base "$BASE")
fi
if [[ "$COMMAND" == "review" && -n "$HEAD" ]]; then
  git cat-file -e "${HEAD}^{commit}" 2>/dev/null || git fetch --no-tags --depth=1 origin "$HEAD"
  RUN_ARGS+=(--head "$HEAD")
fi
if [[ "$IS_AI" == "true" && -n "$FOCUS" ]]; then RUN_ARGS+=(--focus "$FOCUS"); fi
if [[ "$IS_AI" == "true" && -n "$BUDGET" ]]; then RUN_ARGS+=(--budget "$BUDGET"); fi
if [[ "$COMMAND" == "review" && "$FORMAT" == "json" && -z "$OUTPUT" ]]; then
  OUTPUT="repopilot-review.json"
fi
if [[ -n "$OUTPUT" && "$FORMAT" != "sarif" ]]; then RUN_ARGS+=(--output "$OUTPUT"); fi
if [[ "$COMMAND" == "review" ]]; then
  SARIF_OUTPUT="repopilot-review.sarif"
  RUN_ARGS+=(--sarif-output "$SARIF_OUTPUT")
fi
if [[ -n "$RECEIPT" ]]; then RUN_ARGS+=(--receipt "$RECEIPT"); fi

if [[ -n "$ARGS" ]]; then
  read -r -a EXTRA_ARGS <<< "$ARGS"
  RUN_ARGS+=("${EXTRA_ARGS[@]}")
fi

SUMMARY_SOURCE=""
TMP_SUMMARY_OUTPUT=""
REVIEW_SUMMARY_FILE=""

# shellcheck disable=SC2317 # Invoked indirectly by the EXIT trap.
cleanup_action_temp() {
  cleanup_review_temp
  if [[ -n "$TMP_SUMMARY_OUTPUT" && -f "$TMP_SUMMARY_OUTPUT" ]]; then
    rm -f "$TMP_SUMMARY_OUTPUT"
  fi
}

append_job_summary() {
  local status="$1"
  local source_file="${2:-}"

  if [[ -z "${GITHUB_STEP_SUMMARY:-}" ]]; then
    return 0
  fi

  {
    echo "## RepoPilot"
    echo
    printf -- "- **Command:** \`repopilot"
    printf ' %q' "${RUN_ARGS[@]}"
    printf '\`\n'
    echo "- **Exit status:** ${status}"
    if [[ "$FORMAT" == "sarif" ]]; then
      echo "- **SARIF file:** \`${OUTFILE}\`"
    fi
    if [[ -n "$RECEIPT" ]]; then
      echo "- **Receipt:** \`${RECEIPT}\`"
    fi
  } >> "$GITHUB_STEP_SUMMARY"

  if [[ -z "$source_file" || ! -f "$source_file" ]]; then
    return 0
  fi

  {
    echo
    if [[ "$FORMAT" == "console" ]]; then
      echo '```text'
      sed -n '1,200p' "$source_file"
      echo '```'
    else
      sed -n '1,200p' "$source_file"
    fi
  } >> "$GITHUB_STEP_SUMMARY"
}

trap cleanup_action_temp EXIT

OUTFILE="${OUTPUT:-repopilot-results.sarif}"
RUN_STATUS=0
if [[ "$IS_AI" == "true" ]]; then
  if [[ "$FORMAT" != "markdown" ]]; then
    echo "::error::'$COMMAND' emits Markdown and does not accept --format. Use format=auto or format=markdown."
    exit 2
  fi
  if [[ -z "$OUTPUT" && -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    TMP_SUMMARY_OUTPUT="$(mktemp)"
  fi
  set +e
  if [[ -n "$TMP_SUMMARY_OUTPUT" ]]; then
    repopilot "${RUN_ARGS[@]}" | tee "$TMP_SUMMARY_OUTPUT"
    RUN_STATUS=${PIPESTATUS[0]}
    SUMMARY_SOURCE="$TMP_SUMMARY_OUTPUT"
  else
    repopilot "${RUN_ARGS[@]}"
    RUN_STATUS=$?
    SUMMARY_SOURCE="$OUTPUT"
  fi
  set -e
elif [[ "$FORMAT" == "sarif" ]]; then
  set +e
  repopilot "${RUN_ARGS[@]}" --format sarif --output "$OUTFILE"
  RUN_STATUS=$?
  set -e
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    echo "sarif_file=$OUTFILE" >> "$GITHUB_OUTPUT"
  fi
else
  if [[ -z "$OUTPUT" && -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    TMP_SUMMARY_OUTPUT="$(mktemp)"
  fi
  set +e
  if [[ -n "$TMP_SUMMARY_OUTPUT" ]]; then
    repopilot "${RUN_ARGS[@]}" --format "$FORMAT" | tee "$TMP_SUMMARY_OUTPUT"
    RUN_STATUS=${PIPESTATUS[0]}
    SUMMARY_SOURCE="$TMP_SUMMARY_OUTPUT"
  else
    repopilot "${RUN_ARGS[@]}" --format "$FORMAT"
    RUN_STATUS=$?
    SUMMARY_SOURCE="$OUTPUT"
  fi
  set -e
fi

if [[ "$COMMAND" == "review" && "$FORMAT" == "json" && -f "$OUTPUT" ]]; then
  finalize_review_artifacts "$OUTPUT"
  SUMMARY_SOURCE="$REVIEW_SUMMARY_FILE"
fi

if [[ -n "$RECEIPT" && -n "${GITHUB_OUTPUT:-}" ]]; then
  echo "receipt_file=$RECEIPT" >> "$GITHUB_OUTPUT"
fi

if [[ "$RUN_STATUS" -eq 0 ]]; then
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then echo "conclusion=passed" >> "$GITHUB_OUTPUT"; fi
  append_job_summary "passed" "$SUMMARY_SOURCE"
else
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then echo "conclusion=failed" >> "$GITHUB_OUTPUT"; fi
  append_job_summary "failed (exit ${RUN_STATUS})" "$SUMMARY_SOURCE"
fi

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then echo "exit_code=$RUN_STATUS" >> "$GITHUB_OUTPUT"; fi
if [[ "${REPOPILOT_DEFER_FAILURE:-false}" == "true" ]]; then
  exit 0
fi
exit "$RUN_STATUS"
