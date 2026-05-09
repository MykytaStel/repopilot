## Highlights

- `repopilot vibe` generates LLM-ready Markdown from local scan findings.
- `repopilot harden` generates prioritized P0/P1/P2/P3 remediation plans.
- `repopilot prompt` generates paste-ready remediation prompts with RepoPilot context.
- Scan presets (`--preset strict|balanced|lenient`) tune thresholds without editing config.
- `scan --verbose` prints scan and render timing for local performance checks.
- First-party GitHub Action can run `scan`, `review`, `compare`, `vibe`, `harden`, and `prompt`.
- Release publishing is hardened while keeping npm Trusted Publishing separate from the tag workflow.
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

## Vibe Example

```bash
repopilot vibe .
repopilot vibe . --focus security --budget 2k
repopilot harden . --focus security
repopilot prompt . --budget 8k
```

## Links

- README: https://github.com/MykytaStel/repopilot#readme
- Changelog: https://github.com/MykytaStel/repopilot/blob/main/CHANGELOG.md
- React Native docs: https://github.com/MykytaStel/repopilot/blob/main/docs/react-native.md
- GitHub Code Scanning: https://github.com/MykytaStel/repopilot/blob/main/docs/integrations/github-code-scanning.md
