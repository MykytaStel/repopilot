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

## Contract and ownership

<!-- Identify the subsystem and any public behavior or schema affected. -->

- [ ] The change stays within a clear subsystem or ownership boundary
- [ ] CLI, JSON, SARIF, baseline, Action, and MCP compatibility were considered
- [ ] New or changed findings/signals include deterministic evidence and tests
- [ ] No unrelated refactor or generated-file churn is included

## Changelog

- [ ] `CHANGELOG.md` updated (skip for CI-only or doc changes with no user-visible effect)

## Verification

<!-- Paste the commands you ran. -->

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm run release:contract
./scripts/smoke-product.sh
```
