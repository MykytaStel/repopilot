# Contributing to RepoPilot

Thanks for your interest in contributing to RepoPilot.

## Development setup

```bash
git clone https://github.com/MykytaStel/repopilot.git
cd repopilot
cargo build
```

Verify your build works:

```bash
cargo run -- scan .
```

## Running tests

Unit and integration tests:

```bash
cargo test --all
```

Run a specific integration test suite:

```bash
cargo test --test cli_stabilization
cargo test --test cli_release_smoke
cargo test --test explain_cli
```

Run a single test by name:

```bash
cargo test -- test_name_here
```

## Before submitting

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm run test:npm
npm run release:contract
```

Fix formatting automatically with `cargo fmt` (no flags).

## Pull Requests

Please keep PRs focused and small.

Good PRs include:

- clear motivation in the description;
- a focused ownership and behavioral scope;
- tests for new behavior;
- compatibility notes for CLI, JSON, SARIF, baseline, or Action changes;
- documentation updates for user-facing changes.
