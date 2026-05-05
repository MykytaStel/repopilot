## Summary

<!-- What changed in this PR? Keep it short and practical. -->

## Type of change

- [ ] Feature
- [ ] Refactor
- [ ] Bug fix
- [ ] Tests
- [ ] Documentation
- [ ] CI / repository maintenance

## What changed

<!-- List the main changes. -->

-
-
-

## Why

<!-- Why is this change needed? What problem does it solve? -->

## Architecture notes

<!-- Explain module boundaries if this touches architecture. -->

- [ ] No god files introduced
- [ ] Logic is split into focused modules
- [ ] Scanner, audits, output, and report responsibilities remain separated
- [ ] Findings include evidence where applicable

## Changelog

- [ ] `CHANGELOG.md` updated (skip for CI-only or doc changes with no user-visible effect)

## Verification

<!-- Paste the commands you ran. -->

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo run -- scan .
```