# RepoPilot 0.11.0 Release Checklist

RepoPilot 0.11.0 is the signal quality and CI polish release.

Main release promise:

```text
signal quality -> action parity -> release trust
```

## Release Scope

- Keep the stable command surface compatible with 0.10.
- Reduce self-audit P2 noise without hiding P0/P1 risk.
- Keep JSON, SARIF, baseline, and receipt schemas backward-compatible.
- Bring the first-party GitHub Action closer to the CLI flags used in CI.
- Keep runtime commands local-first: no telemetry, source upload, hosted scanner, or LLM API calls.

## Signal Quality Gates

Run the self-audit from the repository root:

```bash
cargo run -- scan . --format json --output /tmp/repopilot-self-audit.json
```

Verify:

- `risk_summary.counts.p0 == 0`
- `risk_summary.counts.p1 == 0`
- `risk_summary.counts.p2 <= 135`
- high/critical findings remain at `0`

Spot-check the medium-or-higher clusters:

```bash
cargo run -- scan . --min-severity medium --format json \
  | jq '.findings | group_by(.rule_id)[] | {rule: .[0].rule_id, count: length}'
```

Expected dominant clusters should stay explainable: long functions, complex files,
large files, and only actionable Rust panic-risk findings.

## Follow-Up Refactor Candidates

Do not block the 0.11.0 release on broad large-file refactors. Track these
hotspots after release unless they become scan stability issues:

- `src/output/markdown.rs`
- `src/output/console.rs`
- `src/commands/scan.rs`
- `src/knowledge/catalog.rs`
- `src/audits/context/model.rs`

## GitHub Action Parity

Validate the first-party action supports:

- `command: doctor`
- `fail-on-priority`
- `min-priority`
- `rule`
- `timing`
- `receipt` only with `command: scan`
- SARIF upload only with `command: scan`

Smoke examples:

```yaml
- uses: MykytaStel/repopilot@v0.11.0
  with:
    command: doctor
    format: markdown
    output: repopilot-doctor.md

- uses: MykytaStel/repopilot@v0.11.0
  with:
    command: scan
    min-priority: p2
    rule: language.rust.panic-risk
    timing: "true"
    upload-sarif: "false"

- uses: MykytaStel/repopilot@v0.11.0
  with:
    command: review
    base: origin/main
    baseline: .repopilot/baseline.json
    fail-on-priority: p1
    upload-sarif: "false"
```

## Required Local Checks

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm test
npm pack --dry-run
cargo package --list
cargo publish --dry-run
./scripts/verify-release.sh
```

Run the product smoke suite against the release binary:

```bash
cargo build --release
./scripts/smoke-product.sh --binary ./target/release/repopilot --repo .
```

## Version Consistency

Before tagging, update and verify:

```bash
grep '^version = "0.11.0"' Cargo.toml
grep '"version": "0.11.0"' package.json
grep '## \[0.11.0\]' CHANGELOG.md
rg '0\.[[]10[]]\.0|release-checklist-0\.[[]10[]]' Cargo.toml package.json action.yml README.md docs .github scripts install.sh Formula
```

The final search should only report historical changelog entries, old release
checklists, and examples intentionally pinned to old versions.
