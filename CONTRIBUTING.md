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
cargo run -- review .
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
npm run test:release-contract
npm run release:contract
```

Fix formatting automatically with `cargo fmt` (no flags).

To opt into a local formatting check before each commit:

```bash
bash scripts/install-dev-hook.sh
```

The installer refuses to replace an existing hook or change a custom
`core.hooksPath`. The hook checks the working tree without rewriting files or
staging changes. It is not an index-only check: review partially staged Rust
changes separately. Git worktrees may share the installed hook. CI remains
the authoritative check for the committed tree, including API-created commits.


## Pull Requests

Please keep PRs focused and small.

Good PRs include:

- clear motivation in the description;
- a focused ownership and behavioral scope;
- tests for new behavior;
- compatibility notes for CLI, JSON, SARIF, baseline, or Action changes;
- documentation updates for user-facing changes.
