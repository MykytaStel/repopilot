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

echo "==> Verify built binary"
./target/release/repopilot --version
./target/release/repopilot scan . --format markdown --output /tmp/repopilot-release-scan.md
./target/release/repopilot doctor . --format markdown --output /tmp/repopilot-release-doctor.md
./target/release/repopilot ai context . --focus security --budget 2k --output /tmp/repopilot-release-vibe.md
./target/release/repopilot ai plan . --focus all --budget 4k --output /tmp/repopilot-release-harden.md
./target/release/repopilot ai prompt . --focus quality --budget 4k --output /tmp/repopilot-release-prompt.md
./target/release/repopilot inspect knowledge --section rules --format markdown --output /tmp/repopilot-release-knowledge.md

test -s /tmp/repopilot-release-scan.md
test -s /tmp/repopilot-release-doctor.md
test -s /tmp/repopilot-release-vibe.md
test -s /tmp/repopilot-release-harden.md
test -s /tmp/repopilot-release-prompt.md
test -s /tmp/repopilot-release-knowledge.md

echo "==> Release verification passed"
