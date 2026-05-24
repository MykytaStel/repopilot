# RepoPilot 0.10.0 Release Checklist

RepoPilot 0.10.0 is the adoption UX release for moving a repository from first
doctor run to a committed baseline, CI gate, and reproducible audit receipt.

Main release promise:

```text
doctor -> scan report + receipt -> baseline -> CI gate -> AI context
```

## Release scope

0.10.0 focuses on making RepoPilot easier to roll out in real repositories:

- Doctor checks for readable config, readable baseline, RepoPilot CI gates, and report/receipt readiness.
- First-class audit receipts through `repopilot scan --receipt`.
- GitHub Action typed `receipt` input and `receipt-file` output.
- 0.x compatibility aliases for AI and inspect workflows while the stable command surface remains grouped.
- Complete registry and Knowledge Engine metadata for all 46 built-in rules.
- Documentation for adoption workflows, receipt JSON, and CI receipt artifact upload.

## Required local checks

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm test
npm pack --dry-run
cargo package --list
./scripts/verify-release.sh
```

All checks must pass before tagging. If checking a dirty release-prep branch before
commit, use `cargo package --allow-dirty --list` only to inspect package contents.

## Manual smoke checks

```bash
cargo run -- --version
cargo run -- doctor . --format markdown --output /tmp/repopilot-doctor.md
cargo run -- scan . --format markdown --output /tmp/repopilot-report.md --receipt /tmp/repopilot-receipt.json
cargo run -- baseline create . --output /tmp/repopilot-baseline-source.json --force
cargo run -- scan . --baseline /tmp/repopilot-baseline-source.json --fail-on new-high --format json --output /tmp/repopilot-baseline.json
cargo run -- ai context . --budget 4k --output /tmp/repopilot-context.md
cargo run -- vibe . --budget 2k --output /tmp/repopilot-legacy-context.md
cargo run -- inspect knowledge --section rules --format json --output /tmp/repopilot-knowledge-rules.json
```

Check that all files exist and are not empty:

```bash
test -s /tmp/repopilot-doctor.md
test -s /tmp/repopilot-report.md
test -s /tmp/repopilot-receipt.json
test -s /tmp/repopilot-baseline-source.json
test -s /tmp/repopilot-baseline.json
test -s /tmp/repopilot-context.md
test -s /tmp/repopilot-legacy-context.md
test -s /tmp/repopilot-knowledge-rules.json
```

## Version consistency

Check:

```bash
grep '^version = "0.10.0"' Cargo.toml
grep '"version": "0.10.0"' package.json
grep '## \[0.10.0\]' CHANGELOG.md
rg '0\.[[]9[]]\.0|release-checklist-0\.[[]9[]]' Cargo.toml package.json action.yml README.md docs .github scripts install.sh Formula
```

The final search should only report historical changelog entries, old release
checklists, and example version text where intentionally retained.

## Publishing order

1. Merge the release preparation PR into `main`.
2. Pull the latest `main`.

```bash
git checkout main
git pull
```

3. Tag the release:

```bash
git tag v0.10.0
git push origin v0.10.0
```

4. Wait for the GitHub Release workflow and npm publishing workflow to finish.
5. Verify GitHub Release assets exist for Linux, macOS, Windows, and `.sha256` files.
6. Verify the configured distribution channels:

```bash
cargo install repopilot --force
npm install -g repopilot@0.10.0
brew tap mykytastel/repopilot
brew install repopilot
repopilot --version
brew test repopilot
```

## Post-release verification

```bash
repopilot doctor . --format markdown --output repopilot-doctor.md
repopilot scan . --format markdown --output repopilot-report.md --receipt .repopilot/receipt.json
repopilot scan . --format sarif --output repopilot.sarif
repopilot ai context . --budget 4k --output repopilot-context.md
```

## Release note summary

RepoPilot 0.10.0 makes adoption easier by turning doctor into a rollout guide,
making audit receipts explicit and schema-versioned, and letting the first-party
GitHub Action publish receipt artifacts from scan jobs.
