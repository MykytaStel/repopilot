# RepoPilot 0.12.0 Release Checklist

RepoPilot 0.12.0 is the core foundation, local feedback, and release trust
sync release.

Main release promise:

```text
core foundation -> local feedback transparency -> release trust sync
```

## Release Scope

- Keep the stable command surface compatible with 0.11.
- Keep JSON, SARIF, baseline, receipt, and scan report schemas backward-compatible.
- Keep runtime commands local-first: no telemetry, source upload, hosted scanner, plugin runtime, external rule execution, or LLM API calls.
- Document the Knowledge Engine lifecycle and local-first learning policy.
- Move workspace scan orchestration out of the CLI command layer without changing behavior.
- Keep `bundled_knowledge()` as the only runtime Knowledge Engine source.
- Add validated local feedback suppressions without hosted scanner, telemetry, or
  arbitrary plugin runtime.
- Keep public distribution channels synchronized at `0.12.0` after publishing.

## Knowledge Engine Gates

Verify:

- every registered rule has metadata;
- every registered rule has a Knowledge Engine applicability entry;
- every registered rule has a non-empty recommendation;
- `inspect knowledge --section rules --format json` keeps the same JSON shape;
- `inspect explain` still evaluates rule decisions from the bundled core pack.

Smoke commands:

```bash
cargo run -- inspect knowledge --section rules --format json --output /tmp/repopilot-knowledge-rules.json
cargo run -- inspect knowledge --section rules --format markdown --output /tmp/repopilot-knowledge-rules.md
cargo run -- inspect explain src/main.rs --rule language.rust.panic-risk --signal rust.unwrap
```

## Workspace Scan Gates

Verify `--workspace` behavior is unchanged:

- fallback warning appears when no workspace packages are found;
- root scan ignores package roots before per-package scans;
- package findings include `workspace_package`;
- duplicate root/package findings are removed;
- workspace hotspot and repeated-cluster overlays still run after merge;
- health score and sorting are recomputed after merge.

Smoke command:

```bash
cargo run -- scan . --workspace --format json --output /tmp/repopilot-workspace.json
```

## Signal Quality Gates

Run the self-audit from the repository root:

```bash
cargo run -- scan . --format json --output /tmp/repopilot-self-audit.json
```

Verify:

- `risk_summary.counts.p0 == 0`
- `risk_summary.counts.p1 == 0`
- high/critical findings remain at `0`
- any P2 increase from 0.11 is explained by a deliberate rule or calibration change

Spot-check medium-or-higher clusters:

```bash
cargo run -- scan . --min-severity medium --format json \
  | jq '.findings | group_by(.rule_id)[] | {rule: .[0].rule_id, count: length}'
```

## Adoption And Feedback Gates

Verify RepoPilot's own repository is adopted:

```bash
repopilot doctor .
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
```

Expected:

- `.repopilotignore` exists;
- `.repopilot/baseline.json` exists and parses;
- CI contains a RepoPilot gate with `--baseline .repopilot/baseline.json --fail-on new-high`;
- default self-scan has no P0/P1/high/critical findings;
- `repopilot doctor .` reports no adoption warnings.

Baseline governance:

- `.repopilot/baseline.json` represents accepted existing debt so teams can
  adopt RepoPilot gradually without blocking on known findings;
- update it only after intentional review, not just to make CI green;
- baseline update PRs should explain why the findings are accepted;
- recommended creation command:
  `repopilot baseline create . --output .repopilot/baseline.json`;
- CI should gate with:
  `repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high`.

Verify local feedback:

```bash
repopilot inspect feedback .
repopilot inspect feedback . --evaluate
repopilot scan . --format json --output /tmp/repopilot-feedback.json
repopilot scan . --ignore-feedback --format json --output /tmp/repopilot-raw.json
```

Expected:

- malformed feedback emits `feedback.parse-failed`;
- invalid entries emit `feedback.invalid-suppression`;
- unmatched suppressions emit `feedback.unmatched-suppressions`;
- scan, review, JSON, Markdown, console, and receipt output expose
  `local_feedback` when feedback is applied.

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
```

Run the product smoke suite against the release binary:

```bash
cargo build --release
./scripts/smoke-product.sh --binary ./target/release/repopilot --repo .
```

## Version Consistency

Before tagging, update and verify:

```bash
grep '^version = "0.12.0"' Cargo.toml
grep '"version": "0.12.0"' package.json
grep '## \[0.12.0\]' CHANGELOG.md
rg -n '0\.11\.0|release-checklist-0\.11' Cargo.toml package.json action.yml README.md docs .github scripts install.sh Formula examples
```

The final search should only report historical changelog entries, old release
checklists, old release announcements, and examples intentionally pinned to old
versions.

## Post-Publish Public Channel Checks

After the tag/release workflows complete, verify the public state:

```bash
npm view repopilot version
cargo search repopilot --limit 5
gh release list --repo MykytaStel/repopilot --limit 5
```

All three should show `0.12.0` as the latest public version before promotion
posts go out.
