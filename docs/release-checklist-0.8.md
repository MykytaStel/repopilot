# RepoPilot 0.8.0 Release Checklist

RepoPilot 0.8.0 is the AI-assisted remediation workflow release.

Main release promise:

```text
scan → ai context → ai plan → ai prompt → fix
```

## Release scope

0.8.0 focuses on making RepoPilot useful for AI-assisted development workflows:

- `repopilot ai context` generates LLM-ready repository context.
- `repopilot ai plan` generates a prioritized remediation plan.
- `repopilot ai prompt` generates a paste-ready AI remediation prompt.
- `scan` supports better input controls and report formats.
- GitHub Action supports `scan`, `review`, `compare`, `ai-context`, `ai-plan`, and `ai-prompt` workflows.

## Required local checks

Run from the repository root:

```bash
./scripts/verify-release.sh
```

This must pass before tagging.

## Manual smoke checks

```bash
cargo run -- --version
cargo run -- scan . --format markdown --output /tmp/repopilot-scan.md
cargo run -- doctor . --format markdown --output /tmp/repopilot-doctor.md
cargo run -- ai context . --focus security --budget 2k --output /tmp/repopilot-vibe.md
cargo run -- ai plan . --focus all --budget 4k --output /tmp/repopilot-harden.md
cargo run -- ai prompt . --focus quality --budget 4k --output /tmp/repopilot-prompt.md
cargo run -- inspect knowledge --section rules --format markdown --output /tmp/repopilot-knowledge.md
```

Check that all files exist and are not empty:

```bash
test -s /tmp/repopilot-scan.md
test -s /tmp/repopilot-doctor.md
test -s /tmp/repopilot-vibe.md
test -s /tmp/repopilot-harden.md
test -s /tmp/repopilot-prompt.md
test -s /tmp/repopilot-knowledge.md
```

## Version consistency

Check:

```bash
grep '^version = "0.8.0"' Cargo.toml
grep '"version": "0.8.0"' package.json
grep '## \[0.8.0\]' CHANGELOG.md
```

## Publishing order

1. Merge the release preparation PR into `main`.
2. Pull the latest `main`.

```bash
git checkout main
git pull
```

3. Tag the release:

```bash
git tag v0.8.0
git push origin v0.8.0
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

7. Publish or verify the npm package after GitHub Release assets are available:

```bash
npm publish
npm install -g repopilot@0.8.0
repopilot --version
```

## Post-release verification

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot doctor . --format markdown --output repopilot-doctor.md
repopilot ai context . --focus security --budget 2k --output repopilot-vibe.md
repopilot ai plan . --focus all --budget 4k --output repopilot-harden.md
repopilot ai prompt . --budget 8k --output repopilot-prompt.md
repopilot inspect knowledge --section rules --format markdown --output repopilot-knowledge.md
```

## Release note summary

RepoPilot 0.8.0 turns repository audits into AI-ready remediation workflows.

It adds practical commands for developers who use Claude Code, Cursor, ChatGPT, or other coding assistants:

- `ai context` for LLM-ready repo context.
- `ai plan` for prioritized fixes.
- `ai prompt` for paste-ready remediation prompts.

The release keeps RepoPilot local-first: code stays on the user's machine, while the output is structured for fast AI-assisted fixing.
