# Contributing to RepoPilot

Thanks for your interest in contributing to RepoPilot.

## Local Setup

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Pull Requests

Please keep PRs focused and small.

Good PRs include:

- clear motivation;
- tests for new behavior;
- documentation updates for user-facing changes.

## Before Submitting

Run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
