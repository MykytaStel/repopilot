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
