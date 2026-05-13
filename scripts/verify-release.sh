#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "==> RepoPilot release verification"

echo "==> Rust formatting"
cargo fmt --all -- --check

echo "==> Rust clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> Rust tests"
cargo test --all

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
cargo package --list

echo "==> Cargo publish dry-run"
cargo publish --dry-run

echo "==> npm wrapper tests"
npm run test:npm

echo "==> npm package dry-run"
npm pack --dry-run

echo "==> Build release binary"
cargo build --release

echo "==> Product readiness smoke suite"
./scripts/smoke-product.sh \
  --binary ./target/release/repopilot \
  --repo . \
  --tmp-dir /tmp/repopilot-release-smoke

echo "==> Release verification passed"
