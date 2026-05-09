## Highlights

- Workspace scanning with per-package risk summaries.
- React Native architecture profile and framework-specific findings.
- React Native dependency health checks.
- Complete rule metadata registry and enriched SARIF output.
- Baseline-aware CI gate for new findings.
- First-party GitHub Action and SARIF output for GitHub Code Scanning.
- Local-first safety layer positioning for AI-assisted and vibe-coded changes.
- npm, cargo, curl, and GitHub Release binary install paths.

## Install

```bash
npm install -g repopilot
cargo install repopilot
curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | sh
```

## React Native Example

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot scan . --workspace --min-severity medium
repopilot review . --base origin/main --baseline .repopilot/baseline.json --fail-on new-high
```

## Links

- README: https://github.com/MykytaStel/repopilot#readme
- Changelog: https://github.com/MykytaStel/repopilot/blob/main/CHANGELOG.md
- React Native docs: https://github.com/MykytaStel/repopilot/blob/main/docs/react-native.md
- GitHub Code Scanning: https://github.com/MykytaStel/repopilot/blob/main/docs/integrations/github-code-scanning.md
