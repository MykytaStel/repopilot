## Highlights

- React Native architecture profile and framework-specific findings.
- Baseline-aware CI gate for new findings.
- SARIF output for GitHub Code Scanning.
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
repopilot review . --base origin/main --baseline .repopilot/baseline.json --fail-on new-high
```

## Links

- README: https://github.com/MykytaStel/repopilot#readme
- Changelog: https://github.com/MykytaStel/repopilot/blob/main/CHANGELOG.md
- React Native docs: https://github.com/MykytaStel/repopilot/blob/main/docs/react-native.md
- GitHub Code Scanning: https://github.com/MykytaStel/repopilot/blob/main/docs/integrations/github-code-scanning.md
