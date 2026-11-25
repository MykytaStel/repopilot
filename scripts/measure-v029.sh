#!/usr/bin/env bash
# Build two clean commits and collect with the candidate's frozen corpus.
# Usage: bash scripts/measure-v029.sh [baseline-ref]
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
    echo "Commit or stash your changes before collecting matched evidence." >&2
    exit 2
fi
baseline=$(git rev-parse --verify "${1:-5b5f0dc676163babb8765ce01270e5bc8cc6bd39}^{commit}")
candidate=$(git rev-parse --verify HEAD)

# Alternate per case; this option selects only the first case's order.
case "${REPOPILOT_BENCHMARK_ORDER:-baseline-first}" in
    baseline-first) collection_order=(baseline candidate); paired_flags=() ;;
    candidate-first) collection_order=(candidate baseline); paired_flags=(--candidate-first) ;;
    *) echo "Invalid REPOPILOT_BENCHMARK_ORDER" >&2; exit 2 ;;
esac
if [[ "$baseline" == "$candidate" ]]; then
    echo "Baseline and candidate must be different commits." >&2
    exit 2
fi

output=$(mktemp -d "${TMPDIR:-/tmp}/repopilot-v029-evidence.XXXXXX")
work=$(mktemp -d "${TMPDIR:-/tmp}/repopilot-v029-work.XXXXXX")
cleanup() {
    git -C "$root" worktree remove --force "$work/baseline" 2>/dev/null || true
    git -C "$root" worktree remove --force "$work/candidate" 2>/dev/null || true
    # Remove only directories allocated by this invocation.
    rm -rf "$work"
    echo "Evidence directory: $output"
}
trap cleanup EXIT
echo "Evidence directory: $output"
# Optional Cargo-managed reuse; always invoke cargo build, never trust an old binary.
target_root="$work"
if [[ -n "${REPOPILOT_BENCHMARK_TARGET_DIR:-}" ]]; then
    mkdir -p "$REPOPILOT_BENCHMARK_TARGET_DIR"
    target_root=$(cd "$REPOPILOT_BENCHMARK_TARGET_DIR" && pwd -P)
fi
git worktree add --detach "$work/baseline" "$baseline"
git worktree add --detach "$work/candidate" "$candidate"

{
    printf 'baseline_commit=%s\ncandidate_commit=%s\n' "$baseline" "$candidate"
    printf 'build=cargo build --release --locked --bin repopilot\n'
    printf 'collection_schedule=alternating-case-v1\n'
    printf 'first_case_order=%s,%s\n' "${collection_order[@]}"
    printf 'target_root=%s\n' "$target_root"
    uname -smr
    rustc -Vv
    cargo -V
    python3 --version
    git --version
} > "$output/environment.txt"

for side in baseline candidate; do
    (
        cd "$work/$side"
        cargo build --release --locked --bin repopilot --target-dir "$target_root/target-$side"
    ) 2>&1 | tee "$output/$side-build.log"
done

# Allow a quiet interval after both builds. No concurrency in measured runs.
cooldown=${REPOPILOT_BENCHMARK_COOLDOWN:-30}
if [[ ! "$cooldown" =~ ^[0-9]+$ ]]; then
    echo "REPOPILOT_BENCHMARK_COOLDOWN must be a nonnegative integer" >&2
    exit 2
fi
printf 'cooldown_seconds=%s\n' "$cooldown" >> "$output/environment.txt"
sleep "$cooldown"

cd "$work/candidate"
# Bash 3.2 (macOS) with nounset needs an explicit nonempty-array guard.
python3 scripts/changeproof_benchmark.py collect-paired \
    --baseline-root "$work/baseline" \
    --repo-root "$work/candidate" \
    --baseline-scanner "$target_root/target-baseline/release/repopilot" \
    --candidate-scanner "$target_root/target-candidate/release/repopilot" \
    --output "$output" \
    ${paired_flags[@]+"${paired_flags[@]}"}

for side in baseline candidate; do
    python3 scripts/changeproof_benchmark.py report \
        --artifact "$output/$side.json" --output "$output/$side.md"
done

python3 scripts/changeproof_benchmark.py gate \
    --baseline-artifact "$output/baseline.json" \
    --candidate-artifact "$output/candidate.json" \
    --output "$output/gate.json"
