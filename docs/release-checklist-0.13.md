# RepoPilot 0.13.0 Release Checklist

RepoPilot 0.13.0 is the breaking cleanup release for the pre-1.0 line.

Main release promise:

```text
engine contract -> stable product commands -> current schema reads -> local-first audit evidence
```

## Release Scope

- Keep the primary CLI families: `scan`, `review`, `baseline`, `compare`, `doctor`, `ai`, `inspect`, `init`, and `cache`.
- Keep runtime commands local-first: no telemetry, source upload, hosted scanner, plugin runtime, external rule execution, or LLM API calls.
- Use `repopilot::api::*` as the supported Rust embedding facade; treat direct internal modules as unsupported implementation detail.
- Require current scan report schema metadata when reading reports for `repopilot compare`.
- Keep rule growth limited to regression fixes only.

## Engine Contract Gates

Verify every rendered finding has:

- stable `id`;
- non-empty `rule_id`;
- non-empty recommendation;
- at least one evidence item;
- risk score, priority, formula version, and risk signals;
- schema-safe serialization in JSON and SARIF.

Smoke commands:

```bash
cargo run -- scan . --format json --output /tmp/repopilot-013-scan.json --receipt /tmp/repopilot-013-receipt.json
cargo run -- scan . --format sarif --output /tmp/repopilot-013.sarif
cargo run -- review . --format json --output /tmp/repopilot-013-review.json
cargo run -- ai context . --budget 2k --output /tmp/repopilot-013-ai-context.md
```

## Breaking-Change Gates

Verify:

- `repopilot compare` accepts current `0.14` scan reports;
- `repopilot compare` rejects pre-current scan report shapes clearly;
- active docs describe the current command surface, not removed 0.x aliases;
- schema migration docs state that historical report compatibility belongs in downstream consumers when needed.

## Required Local Checks

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm run test:npm
npm pack --dry-run
cargo package --list
cargo publish --dry-run
./scripts/verify-release.sh
cargo build --release
./scripts/smoke-product.sh --binary ./target/release/repopilot --repo .
```

## Version Consistency

Before tagging, verify:

```bash
grep '^version = "0.13.0"' Cargo.toml
grep '"version": "0.13.0"' package.json
grep '## \[0.13.0\]' CHANGELOG.md || grep '## \[Unreleased\]' CHANGELOG.md
rg -n '0\.12\.0|release-checklist-0\.12' Cargo.toml package.json action.yml README.md docs .github scripts install.sh Formula examples
```

The final search should only report historical changelog entries, old release
checklists, old release announcements, and examples intentionally pinned to old
versions.
