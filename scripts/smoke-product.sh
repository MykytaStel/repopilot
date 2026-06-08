#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

BINARY=""
REPO_PATH="$ROOT_DIR"
TMP_DIR=""
TMP_CREATED=0
KEEP_TMP=0

usage() {
  cat <<'EOF'
Usage: scripts/smoke-product.sh [OPTIONS]

Run product-readiness smoke checks against RepoPilot.

Options:
  --binary PATH      Use an already built repopilot binary.
  --repo PATH        Repository or project path to smoke test. Defaults to this repo.
  --tmp-dir PATH     Directory for generated smoke artifacts. Defaults to mktemp.
  --keep-tmp         Keep generated smoke artifacts when using a temporary directory.
  -h, --help         Show this help message.

Examples:
  scripts/smoke-product.sh
  scripts/smoke-product.sh --binary ./target/release/repopilot
  scripts/smoke-product.sh --repo /path/to/project --keep-tmp
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)
      BINARY="${2:-}"
      shift 2
      ;;
    --repo)
      REPO_PATH="${2:-}"
      shift 2
      ;;
    --tmp-dir)
      TMP_DIR="${2:-}"
      shift 2
      ;;
    --keep-tmp)
      KEEP_TMP=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$REPO_PATH" ]]; then
  echo "--repo cannot be empty" >&2
  exit 2
fi

REPO_PATH="$(cd "$REPO_PATH" && pwd)"

if [[ -z "$BINARY" ]]; then
  if [[ ! -x "$ROOT_DIR/target/debug/repopilot" ]]; then
    echo "==> Building debug binary for product smoke checks" >&2
    cargo build --manifest-path "$ROOT_DIR/Cargo.toml"
  fi
  BINARY="$ROOT_DIR/target/debug/repopilot"
else
  case "$BINARY" in
    /*) ;;
    *)
      BINARY="$(cd "$(dirname "$BINARY")" && pwd)/$(basename "$BINARY")"
      ;;
  esac
fi

if [[ ! -x "$BINARY" ]]; then
  echo "RepoPilot binary is not executable: $BINARY" >&2
  exit 3
fi

if [[ -z "$TMP_DIR" ]]; then
  TMP_DIR="$(mktemp -d)"
  TMP_CREATED=1
else
  rm -rf "$TMP_DIR"
  mkdir -p "$TMP_DIR"
  TMP_DIR="$(cd "$TMP_DIR" && pwd)"
fi

cleanup() {
  if [[ "$TMP_CREATED" -eq 1 && "$KEEP_TMP" -eq 0 ]]; then
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT

run_repopilot() {
  echo "==> repopilot $*" >&2
  (
    cd "$REPO_PATH"
    "$BINARY" "$@"
  )
}

run_repopilot_in() {
  local cwd="$1"
  shift
  echo "==> repopilot $* (cwd: $cwd)" >&2
  (
    cd "$cwd"
    "$BINARY" "$@"
  )
}

assert_non_empty() {
  local file="$1"
  if [[ ! -s "$file" ]]; then
    echo "Expected non-empty file: $file" >&2
    exit 3
  fi
}

assert_contains() {
  local file="$1"
  local expected="$2"

  if ! grep -Fq "$expected" "$file"; then
    echo "Expected '$file' to contain: $expected" >&2
    echo "---- file content ----" >&2
    cat "$file" >&2 || true
    echo "----------------------" >&2
    exit 3
  fi
}

validate_json_if_possible() {
  local file="$1"

  if command -v python3 >/dev/null 2>&1; then
    python3 -m json.tool "$file" >/dev/null
  fi
}

assert_no_contract_violations() {
  local file="$1"

  if ! grep -Eq '"contract_violations"[[:space:]]*:[[:space:]]*0' "$file"; then
    echo "Expected self-scan report to have zero finding contract violations: $file" >&2
    echo "---- file content ----" >&2
    cat "$file" >&2 || true
    echo "----------------------" >&2
    exit 3
  fi
}

assert_json_number_field() {
  local file="$1"
  local field="$2"
  local expected="$3"

  if command -v python3 >/dev/null 2>&1; then
    python3 - "$file" "$field" "$expected" <<'PY'
import json
import sys

path, field, expected_raw = sys.argv[1], sys.argv[2], sys.argv[3]
expected = int(expected_raw)

with open(path, encoding="utf-8") as handle:
    data = json.load(handle)

value = data.get(field)
if value != expected:
    raise SystemExit(f"Expected {path} field {field} to be {expected}, got {value!r}")
PY
    return
  fi

  if ! grep -Eq "\"$field\"[[:space:]]*:[[:space:]]*$expected([,}])" "$file"; then
    echo "Expected '$file' JSON field '$field' to be: $expected" >&2
    echo "---- file content ----" >&2
    cat "$file" >&2 || true
    echo "----------------------" >&2
    exit 3
  fi
}

echo "==> RepoPilot product smoke suite"
echo "Binary: $BINARY"
echo "Repo:   $REPO_PATH"
echo "Tmp:    $TMP_DIR"
echo

run_repopilot --version > "$TMP_DIR/version.txt"
assert_non_empty "$TMP_DIR/version.txt"

run_repopilot --help > "$TMP_DIR/help.txt"
assert_non_empty "$TMP_DIR/help.txt"
assert_contains "$TMP_DIR/help.txt" "scan"
assert_contains "$TMP_DIR/help.txt" "review"
assert_contains "$TMP_DIR/help.txt" "ai"

run_repopilot init --force --path "$TMP_DIR/repopilot.toml"
assert_non_empty "$TMP_DIR/repopilot.toml"

run_repopilot scan . --format json --output "$TMP_DIR/scan.json" --receipt "$TMP_DIR/receipt.json"
assert_non_empty "$TMP_DIR/scan.json"
validate_json_if_possible "$TMP_DIR/scan.json"
assert_no_contract_violations "$TMP_DIR/scan.json"

# RepoPilot self-scan signal-quality gate.
if command -v python3 >/dev/null 2>&1 && [[ -f "$ROOT_DIR/scripts/check-signal-quality.py" ]]; then
  python3 "$ROOT_DIR/scripts/check-signal-quality.py" \
    --scan-json "$TMP_DIR/scan.json" \
    --output-json "$TMP_DIR/signal-quality.json"
  assert_non_empty "$TMP_DIR/signal-quality.json"
fi
assert_non_empty "$TMP_DIR/receipt.json"
validate_json_if_possible "$TMP_DIR/receipt.json"

run_repopilot scan . --format json --min-priority p2 --rule language.rust.panic-risk --timing --output "$TMP_DIR/scan-rule-priority-timing.json"
assert_non_empty "$TMP_DIR/scan-rule-priority-timing.json"
validate_json_if_possible "$TMP_DIR/scan-rule-priority-timing.json"

PRIORITY_GATE_PROJECT="$TMP_DIR/priority-gate-project"
mkdir -p "$PRIORITY_GATE_PROJECT/src" "$PRIORITY_GATE_PROJECT/tests"
printf 'pub fn ok() -> bool { true }\n' > "$PRIORITY_GATE_PROJECT/src/lib.rs"
printf '#[test]\nfn ok() { assert!(priority_gate_project::ok()); }\n' > "$PRIORITY_GATE_PROJECT/tests/lib_test.rs"
run_repopilot_in "$PRIORITY_GATE_PROJECT" scan . --format json --fail-on-priority p0 --output "$TMP_DIR/scan-priority-gate.json"
assert_non_empty "$TMP_DIR/scan-priority-gate.json"
validate_json_if_possible "$TMP_DIR/scan-priority-gate.json"

BASELINE_ADOPTION_PROJECT="$TMP_DIR/baseline-adoption-project"
mkdir -p "$BASELINE_ADOPTION_PROJECT/src" "$BASELINE_ADOPTION_PROJECT/tests"
SMOKE_SECRET_KEY='API_KEY'
SMOKE_SECRET_VALUE='abc12345'
printf 'const %s: &str = "%s";\n' "$SMOKE_SECRET_KEY" "$SMOKE_SECRET_VALUE" > "$BASELINE_ADOPTION_PROJECT/src/config.rs"
printf 'fn covers_config() {}\n' > "$BASELINE_ADOPTION_PROJECT/tests/config.rs"
run_repopilot_in "$BASELINE_ADOPTION_PROJECT" baseline create . --output .repopilot/baseline.json > "$TMP_DIR/baseline-create.txt"
assert_non_empty "$TMP_DIR/baseline-create.txt"
assert_non_empty "$BASELINE_ADOPTION_PROJECT/.repopilot/baseline.json"
validate_json_if_possible "$BASELINE_ADOPTION_PROJECT/.repopilot/baseline.json"
run_repopilot_in "$BASELINE_ADOPTION_PROJECT" scan . --baseline .repopilot/baseline.json --fail-on new-high --format json --output "$TMP_DIR/baseline-pass.json"
assert_non_empty "$TMP_DIR/baseline-pass.json"
validate_json_if_possible "$TMP_DIR/baseline-pass.json"
printf 'const %s: &str = "%s";\n' "$SMOKE_SECRET_KEY" "$SMOKE_SECRET_VALUE" > "$BASELINE_ADOPTION_PROJECT/src/creds.rs"
if run_repopilot_in "$BASELINE_ADOPTION_PROJECT" scan . --baseline .repopilot/baseline.json --fail-on new-high > "$TMP_DIR/baseline-fail.txt" 2> "$TMP_DIR/baseline-fail.err"; then
  echo "Expected baseline new-high gate to fail after adding a new high-risk finding" >&2
  exit 3
fi
assert_contains "$TMP_DIR/baseline-fail.txt" "CI gate: failed (new-high)"

run_repopilot scan . --format markdown --output "$TMP_DIR/scan.md"
assert_non_empty "$TMP_DIR/scan.md"

run_repopilot scan . --format sarif --output "$TMP_DIR/scan.sarif"
assert_non_empty "$TMP_DIR/scan.sarif"
validate_json_if_possible "$TMP_DIR/scan.sarif"
assert_contains "$TMP_DIR/scan.sarif" '"version": "2.1.0"'

if git -C "$REPO_PATH" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  run_repopilot review . --format markdown --output "$TMP_DIR/review.md"
  assert_non_empty "$TMP_DIR/review.md"
else
  echo "==> Skipping review smoke check because repo path is not inside a Git worktree" >&2
fi

run_repopilot ai context . --focus security --budget 2k --output "$TMP_DIR/ai-context.md"
assert_non_empty "$TMP_DIR/ai-context.md"

echo
echo "==> Product smoke suite passed"
echo "Artifacts: $TMP_DIR"
