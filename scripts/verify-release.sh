#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VERIFY_TMP_DIR="$(mktemp -d)"
cleanup_verify_tmp() {
  rm -rf "$VERIFY_TMP_DIR"
}
trap cleanup_verify_tmp EXIT

echo "==> RepoPilot release verification"

echo "==> Rust formatting"
cargo fmt --all -- --check

echo "==> Rust clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> Rust tests"
cargo test --all

echo "==> Release contract"
python3 scripts/release-contract.py check

echo "==> Release contract tests"
python3 -m unittest discover -s scripts/tests -p 'test_*.py'

echo "==> Rust dependency security audit"
cargo audit

echo "==> Cargo dependency policy"
cargo deny check advisories licenses

echo "==> Shell script syntax"
scripts=()
if [[ -f install.sh ]]; then
  scripts+=(install.sh)
fi
if [[ -d scripts ]]; then
  while IFS= read -r -d '' script; do
    scripts+=("$script")
  done < <(find scripts -maxdepth 1 -type f -name '*.sh' -print0)
fi

if ((${#scripts[@]} > 0)); then
  for script in "${scripts[@]}"; do
    bash -n "$script"
  done

  echo "==> ShellCheck"
  shellcheck "${scripts[@]}"
else
  echo "==> No shell scripts found"
fi

echo "==> GitHub Actions validation"
workflows=()
if [[ -d .github/workflows ]]; then
  while IFS= read -r -d '' workflow; do
    workflows+=("$workflow")
  done < <(find .github/workflows -maxdepth 1 -type f \( -name '*.yml' -o -name '*.yaml' \) -print0)
fi

if ((${#workflows[@]} > 0)); then
  actionlint "${workflows[@]}"
else
  echo "==> No GitHub Actions workflows found"
fi

echo "==> CLI release smoke tests"
cargo test --test cli_release_smoke

echo "==> Cargo package contents"
cargo package --list --allow-dirty

echo "==> Cargo publish dry-run"
cargo publish --dry-run --allow-dirty

echo "==> npm wrapper tests"
npm run test:npm

echo "==> npm package dry-run"
npm pack --dry-run

echo "==> Build release binary"
cargo build --release

echo "==> Self-review signal quality"
REVIEW_BASE="$(git tag --sort=-version:refname --merged HEAD | head -n 1 || true)"
if [[ -z "$(git status --porcelain)" ]] &&
  git describe --tags --exact-match HEAD >/dev/null 2>&1; then
  REVIEW_BASE="$(git tag --sort=-version:refname --merged HEAD | sed -n '2p')"
fi
if [[ -n "$REVIEW_BASE" ]]; then
  ./target/release/repopilot review . \
    --base "$REVIEW_BASE" \
    --format json \
    --output "$VERIFY_TMP_DIR/self-review.json"
  python3 scripts/check-review-quality.py \
    --review-json "$VERIFY_TMP_DIR/self-review.json"
else
  echo "==> No previous release tag found; skipping self-review quality replay"
fi

echo "==> Self-scan signal quality"
./target/release/repopilot scan . --format json --output "$VERIFY_TMP_DIR/self-scan.json"
if command -v python3 >/dev/null 2>&1 && [[ -f scripts/check-signal-quality.py ]]; then
  python3 scripts/check-signal-quality.py \
    --scan-json "$VERIFY_TMP_DIR/self-scan.json" \
    --output-json "$VERIFY_TMP_DIR/signal-quality.json"
fi

echo "==> Product readiness smoke suite"
./scripts/smoke-product.sh \
  --binary ./target/release/repopilot \
  --repo . \
  --tmp-dir /tmp/repopilot-release-smoke

echo "==> Release verification passed"
