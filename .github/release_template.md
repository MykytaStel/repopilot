v0.9 makes RepoPilot ready for practical local-first repository audits across developer machines and CI. Run `repopilot ai context .`, paste the output into Claude Code, ChatGPT, or Cursor, and start fixing with evidence-backed context. No source code leaves your machine.

## Highlights

- **`repopilot ai context`** — scans your repo and formats findings as LLM-ready Markdown.
- **`repopilot ai plan`** — emits a prioritized remediation plan with locations, rule IDs, and verification commands.
- **`repopilot ai prompt`** — generates a paste-ready AI remediation prompt with full RepoPilot context.
- Product docs now cover install paths, security model, AI workflows, language support, and configuration.
- First-party GitHub Action supports typed inputs for scan, review, compare, and AI workflows.
- npm, cargo, Homebrew, curl, and GitHub Release binary install paths are documented and smoke-checked.

## Install

```bash
npm install -g repopilot
cargo install repopilot
brew tap mykytastel/repopilot
brew install repopilot
curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | bash
```

## React Native Example

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot scan . --workspace --min-severity medium
repopilot review . --base origin/main --baseline .repopilot/baseline.json --fail-on new-high
```

## AI Workflow Example

```bash
repopilot ai context .
repopilot ai context . --focus security --budget 2k
repopilot ai plan . --focus security
repopilot ai prompt . --budget 8k
```

## Links

- README: https://github.com/MykytaStel/repopilot#readme
- Changelog: https://github.com/MykytaStel/repopilot/blob/main/CHANGELOG.md
- Install docs: https://github.com/MykytaStel/repopilot/blob/main/docs/install.md
- Security docs: https://github.com/MykytaStel/repopilot/blob/main/docs/security.md
- React Native docs: https://github.com/MykytaStel/repopilot/blob/main/docs/react-native.md
- GitHub Code Scanning: https://github.com/MykytaStel/repopilot/blob/main/docs/integrations/github-code-scanning.md
