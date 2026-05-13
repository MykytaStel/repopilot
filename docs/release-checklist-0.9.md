# RepoPilot 0.9.0 Release Checklist

RepoPilot 0.9.0 is the product-readiness release for local-first repository
audits, AI-assisted remediation workflows, and cross-platform distribution.

Main release promise:

```text
install -> scan -> ai context -> ai plan -> ai prompt -> review
```

## Release scope

0.9.0 focuses on making RepoPilot easier to trust, install, and use:

- Stable `repopilot ai context`, `repopilot ai plan`, and `repopilot ai prompt` workflows.
- Product documentation for install, security, configuration, language support, and AI workflows.
- GitHub Action typed inputs for `scan`, `review`, `compare`, and AI workflows.
- Cross-platform release paths through crates.io, npm, GitHub Releases, Homebrew, and curl.
- Release smoke checks that cover first-run, scan, review, AI, inspect, and compatibility commands.

## Required local checks

Run from the repository root:

```bash
cargo fmt --check
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
cargo run -- scan . --format markdown --output /tmp/repopilot-scan.md
cargo run -- doctor . --format markdown --output /tmp/repopilot-doctor.md
cargo run -- ai context . --focus security --budget 2k --output /tmp/repopilot-context.md
cargo run -- ai plan . --focus all --budget 4k --output /tmp/repopilot-plan.md
cargo run -- ai prompt . --focus quality --budget 4k --output /tmp/repopilot-prompt.md
cargo run -- inspect knowledge --section rules --format markdown --output /tmp/repopilot-knowledge.md
```

Check that all files exist and are not empty:

```bash
test -s /tmp/repopilot-scan.md
test -s /tmp/repopilot-doctor.md
test -s /tmp/repopilot-context.md
test -s /tmp/repopilot-plan.md
test -s /tmp/repopilot-prompt.md
test -s /tmp/repopilot-knowledge.md
```

## Version consistency

Check:

```bash
grep '^version = "0.9.0"' Cargo.toml
grep '"version": "0.9.0"' package.json
grep '## \[0.9.0\]' CHANGELOG.md
rg '0\.[[]8[]]\.0|release-checklist-0\.[[]8[]]|roadmap-v0\.[[]7[]]' Cargo.toml package.json action.yml README.md docs .github scripts install.sh Formula
```

The final search should only report historical changelog entries for the previous release.

## Publishing order

1. Merge the release preparation PR into `main`.
2. Pull the latest `main`.

```bash
git checkout main
git pull
```

3. Tag the release:

```bash
git tag v0.9.0
git push origin v0.9.0
```

4. Wait for the GitHub Release workflow to finish.

5. Verify GitHub Release assets exist for:

- Linux x86_64
- Linux arm64
- macOS x86_64
- macOS arm64
- Windows x86_64
- `.sha256` files

6. Verify the crates.io package:

```bash
cargo install repopilot --force
repopilot --version
```

7. Verify npm after GitHub Release assets are available:

```bash
npm install -g repopilot@0.9.0
repopilot --version
```

8. Verify Homebrew and curl:

```bash
brew tap mykytastel/repopilot
brew install repopilot
brew test repopilot
curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | bash
```

## Post-release verification

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot doctor . --format markdown --output repopilot-doctor.md
repopilot ai context . --focus security --budget 2k --output repopilot-context.md
repopilot ai plan . --focus all --budget 4k --output repopilot-plan.md
repopilot ai prompt . --budget 8k --output repopilot-prompt.md
repopilot inspect knowledge --section rules --format markdown --output repopilot-knowledge.md
```

## Release note summary

RepoPilot 0.9.0 makes local-first repository audits ready for practical adoption
across developer machines and CI.

It packages the AI-assisted remediation workflow with clearer docs, stable install
paths, release checks, and cross-platform distribution support while keeping code
on the user's machine.
