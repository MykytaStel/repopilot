# RepoPilot 0.13.0 Release Checklist

RepoPilot 0.13.0 is the breaking cleanup release for the pre-1.0 line.

Main release promise:

```text
finding contract -> rule lifecycle -> signal quality -> risk-v3 -> local-first audit evidence
```

## Release Scope

- Keep the primary CLI families: `scan`, `review`, `baseline`, `compare`, `doctor`, `ai`, `inspect`, `init`, and `cache`.
- Keep runtime commands local-first: no telemetry, source upload, hosted scanner, plugin runtime, external rule execution, or LLM API calls.
- Use `repopilot::api::*` as the supported Rust embedding facade; treat direct internal modules as unsupported implementation detail.
- Require current scan report schema metadata when reading reports for `repopilot compare`.
- Keep rule growth limited to regression fixes only.
- Keep finding validation, rule evaluation, and signal quality local-only: no telemetry, source upload, hosted scanner, plugin execution, or LLM API calls.

## Engine Contract Gates

Verify every rendered finding has:

- stable `id`;
- non-empty `rule_id`;
- non-empty title and description;
- non-empty recommendation;
- at least one evidence item;
- risk score, priority, formula version, and risk signals;
- provenance with detector, lifecycle, signal source, and scope;
- schema-safe serialization in JSON and SARIF.

Rule metadata gates:

- every registered rule has non-empty title, description, recommendation;
- every registered rule has lifecycle, signal source, and default confidence;
- high/critical findings have docs URLs after enrichment;
- `repopilot inspect rules`, `inspect rule`, and `inspect eval-rules` pass locally.
- `scripts/smoke-product.sh` verifies the self-scan JSON report has zero finding
  contract violations without requiring `jq` or other optional JSON tools.

Smoke commands:

```bash
cargo run -- scan . --format json --output /tmp/repopilot-013-scan.json --receipt /tmp/repopilot-013-receipt.json
cargo run -- scan . --format sarif --output /tmp/repopilot-013.sarif
cargo run -- review . --format json --output /tmp/repopilot-013-review.json
cargo run -- ai context . --budget 2k --output /tmp/repopilot-013-ai-context.md
cargo run -- inspect rules --format json --output /tmp/repopilot-013-rules.json
cargo run -- inspect eval-rules --format json --output /tmp/repopilot-013-rule-eval.json
```

Optimization/stabilization verification:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo run -- scan . --timing
cargo run -- scan . --format json --output /tmp/repopilot-013-optimized.json
cargo run -- inspect rules --format json --output /tmp/repopilot-rules.json
cargo run -- inspect eval-rules --format json --output /tmp/repopilot-rule-eval.json
./scripts/smoke-product.sh --binary ./target/release/repopilot --repo .
```

## Breaking-Change Gates

Verify:

- `repopilot compare` accepts current `0.15` scan reports;
- `repopilot compare` rejects pre-current scan report shapes clearly;
- JSON reports include `signal_quality`, finding `provenance`, `risk.signals[].source`, and `risk-v3`;
- JSON reports include `scan_timings.contract_validation_us` when scan timings are present;
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
