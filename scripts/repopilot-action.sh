#!/usr/bin/env bash
set -euo pipefail

COMMAND="${INPUT_COMMAND:-scan}"
FORMAT="${INPUT_FORMAT:-auto}"
PATH_INPUT="${INPUT_PATH:-.}"
CONFIG="${INPUT_CONFIG:-}"
BASELINE="${INPUT_BASELINE:-}"
FAIL_ON="${INPUT_FAIL_ON:-}"
FAIL_ON_PRIORITY="${INPUT_FAIL_ON_PRIORITY:-}"
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

RUN_ARGS=()
case "$COMMAND" in
  scan) RUN_ARGS=(scan "$PATH_INPUT") ;;
  review) RUN_ARGS=(review "$PATH_INPUT") ;;
  compare) RUN_ARGS=(compare) ;;
  doctor) RUN_ARGS=(doctor "$PATH_INPUT") ;;
  ai-context) RUN_ARGS=(ai context "$PATH_INPUT") ;;
  ai-plan) RUN_ARGS=(ai plan "$PATH_INPUT") ;;
  ai-prompt) RUN_ARGS=(ai prompt "$PATH_INPUT") ;;
  *)
    echo "::error::Unsupported command '$COMMAND'. Expected scan, review, compare, doctor, ai-context, ai-plan, or ai-prompt."
    exit 2
    ;;
esac

IS_AI=false
case "$COMMAND" in
  ai-context|ai-plan|ai-prompt) IS_AI=true ;;
esac

if [[ "$FORMAT" == "auto" ]]; then
  if [[ "$COMMAND" == "scan" ]]; then
    FORMAT="sarif"
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

if [[ -n "$CONFIG" && "$COMMAND" != "compare" ]]; then RUN_ARGS+=(--config "$CONFIG"); fi
if [[ "$COMMAND" == "scan" || "$COMMAND" == "review" ]]; then
  if [[ -n "$BASELINE" ]]; then RUN_ARGS+=(--baseline "$BASELINE"); fi
  if [[ -n "$FAIL_ON" ]]; then RUN_ARGS+=(--fail-on "$FAIL_ON"); fi
  if [[ -n "$FAIL_ON_PRIORITY" ]]; then RUN_ARGS+=(--fail-on-priority "$FAIL_ON_PRIORITY"); fi
  if [[ -n "$MIN_SEVERITY" ]]; then RUN_ARGS+=(--min-severity "$MIN_SEVERITY"); fi
  if [[ -n "$MIN_PRIORITY" ]]; then RUN_ARGS+=(--min-priority "$MIN_PRIORITY"); fi
fi
if [[ "$COMMAND" == "scan" && -n "$RULE_INPUT" ]]; then
  read -r -a RULES <<< "$RULE_INPUT"
  for RULE in "${RULES[@]}"; do
    RUN_ARGS+=(--rule "$RULE")
  done
fi
if [[ "$COMMAND" == "scan" && "$TIMING" == "true" ]]; then RUN_ARGS+=(--timing); fi
if [[ "$COMMAND" == "review" && -n "$BASE" ]]; then RUN_ARGS+=(--base "$BASE"); fi
if [[ "$COMMAND" == "review" && -n "$HEAD" ]]; then RUN_ARGS+=(--head "$HEAD"); fi
if [[ "$IS_AI" == "true" && -n "$FOCUS" ]]; then RUN_ARGS+=(--focus "$FOCUS"); fi
if [[ "$IS_AI" == "true" && -n "$BUDGET" ]]; then RUN_ARGS+=(--budget "$BUDGET"); fi
if [[ -n "$OUTPUT" && "$FORMAT" != "sarif" ]]; then RUN_ARGS+=(--output "$OUTPUT"); fi
if [[ -n "$RECEIPT" ]]; then RUN_ARGS+=(--receipt "$RECEIPT"); fi

if [[ -n "$ARGS" ]]; then
  read -r -a EXTRA_ARGS <<< "$ARGS"
  RUN_ARGS+=("${EXTRA_ARGS[@]}")
fi

OUTFILE="${OUTPUT:-repopilot-results.sarif}"
RUN_STATUS=0
if [[ "$IS_AI" == "true" ]]; then
  if [[ "$FORMAT" != "markdown" ]]; then
    echo "::error::'$COMMAND' emits Markdown and does not accept --format. Use format=auto or format=markdown."
    exit 2
  fi
  set +e
  repopilot "${RUN_ARGS[@]}"
  RUN_STATUS=$?
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
  set +e
  repopilot "${RUN_ARGS[@]}" --format "$FORMAT"
  RUN_STATUS=$?
  set -e
fi

if [[ -n "$RECEIPT" && -n "${GITHUB_OUTPUT:-}" ]]; then
  echo "receipt_file=$RECEIPT" >> "$GITHUB_OUTPUT"
fi

exit "$RUN_STATUS"
