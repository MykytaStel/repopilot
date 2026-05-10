v0.8 brings AI-first analysis to RepoPilot. Run `repopilot vibe .`, paste the output into Claude Code or ChatGPT, and start fixing — no code leaves your machine.

## Highlights

- **`repopilot vibe`** — scans your repo and formats findings as LLM-ready Markdown. Paste directly into Claude Code, Cursor, or ChatGPT to start remediating.
- **`repopilot harden`** — emits a prioritized P0/P1/P2/P3 remediation plan with locations, rule IDs, and verification commands.
- **`repopilot prompt`** — generates a paste-ready AI remediation prompt with full RepoPilot context.
- Scan presets (`--preset strict|balanced|lenient`) tune all thresholds in one flag without touching config.
- `scan --verbose` prints scan and render timing to stderr.
- First-party GitHub Action supports `vibe`, `harden`, and `prompt` commands.
- npm, cargo, Homebrew, curl, and GitHub Release binary install paths.

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
