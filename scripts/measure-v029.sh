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

# Counterbalance the collection order on a second run to expose host drift.
case "${REPOPILOT_BENCHMARK_ORDER:-baseline-first}" in
    baseline-first) collection_order=(baseline candidate) ;;
    candidate-first) collection_order=(candidate baseline) ;;
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
git worktree add --detach "$work/baseline" "$baseline"
git worktree add --detach "$work/candidate" "$candidate"

{
    printf 'baseline_commit=%s\ncandidate_commit=%s\n' "$baseline" "$candidate"
    printf 'build=cargo build --release --locked --bin repopilot\n'
    printf 'collection_order=%s,%s\n' "${collection_order[@]}"
    uname -smr
    rustc -Vv
    cargo -V
    python3 --version
    git --version
} > "$output/environment.txt"

for side in baseline candidate; do
    (
        cd "$work/$side"
        cargo build --release --locked --bin repopilot --target-dir "$work/target-$side"
    ) 2>&1 | tee "$output/$side-build.log"
done

# Both collections use exactly the same manifest, fixture tree, collector and
# timeout, even if the baseline predates the expanded corpus.
cd "$work/candidate"
for side in "${collection_order[@]}"; do
    python3 scripts/changeproof_benchmark.py collect \
        --repo-root "$work/$side" \
        --scanner "$work/target-$side/release/repopilot" \
        --output "$output/$side.json"
    python3 scripts/changeproof_benchmark.py report \
        --artifact "$output/$side.json" --output "$output/$side.md"
done

python3 scripts/changeproof_benchmark.py gate \
    --baseline-artifact "$output/baseline.json" \
    --candidate-artifact "$output/candidate.json" \
    --output "$output/gate.json"
