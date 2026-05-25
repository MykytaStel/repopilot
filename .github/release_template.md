RepoPilot 0.13 is the trust and usability release: cleaner default output,
stronger rule quality checks, safer baseline adoption, and better evidence for
CI and AI-assisted remediation.

## Highlights

- Cleaner default scans keep broad maintainability and testing noise out of the normal CI path.
- `repopilot inspect eval-rules` validates fixture coverage, stable finding IDs, and finding contracts.
- Baseline adoption is first-class: create a baseline, then fail only on new high-risk findings.
- Reports include better evidence for CI: compact console, full diagnostics, JSON, SARIF, Markdown, receipts, and signal quality.
- AI context uses the same local product scan path and default visibility policy as normal scans.

## Install

```bash
npm install -g repopilot
cargo install repopilot
brew tap mykytastel/repopilot
brew install repopilot
curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | bash
```

## Try It

```bash
repopilot scan .
repopilot baseline create . --output .repopilot/baseline.json
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
repopilot ai context . --budget 4k
repopilot inspect eval-rules --format json
```

## CI Evidence

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot scan . --format json --output repopilot-report.json --receipt .repopilot/receipt.json
repopilot scan . --format sarif --output repopilot.sarif
```

## Links

- README: https://github.com/MykytaStel/repopilot#readme
- Changelog: https://github.com/MykytaStel/repopilot/blob/main/CHANGELOG.md
- Install docs: https://github.com/MykytaStel/repopilot/blob/main/docs/install.md
- Rule quality gate: https://github.com/MykytaStel/repopilot/blob/main/docs/rule-quality-gate.md
- GitHub Code Scanning: https://github.com/MykytaStel/repopilot/blob/main/docs/integrations/github-code-scanning.md
