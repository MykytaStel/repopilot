# RepoPilot 0.12.0 Release Announcement

RepoPilot 0.12.0 is the trust release: local-first scans, AI-ready context,
baseline adoption, receipts, SARIF, and a first-party GitHub Action that can gate
pull requests without uploading source code.

## Suggested Post

RepoPilot 0.12.0 is ready.

This release is for teams that want a fast repository safety layer before code
goes into CI or an AI coding session:

- scan locally with no telemetry, no hosted scanner, and no source upload;
- paste `ai context`, `ai plan`, or `ai prompt` into Claude Code, ChatGPT, Cursor,
  or another coding assistant;
- adopt gradually with `.repopilot/baseline.json` and fail CI only on new risk;
- publish SARIF, receipts, and GitHub Action job summaries for pull requests;
- keep local suppressions visible through `local_feedback` metadata.

Try it:

```bash
repopilot doctor .
repopilot baseline create .
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
repopilot ai context . --budget 4k
```

GitHub Actions:

```yaml
- uses: MykytaStel/repopilot@v0.12.0
  with:
    command: scan
    baseline: .repopilot/baseline.json
    fail-on: new-high
    receipt: repopilot-receipt.json
```

Best first audiences:

- AI developers who want safer local context before asking an assistant to edit;
- CI teams that need baseline-aware gates, SARIF, receipts, and no source upload;
- React Native and Expo teams checking New Architecture, Hermes, deprecated APIs,
  and architecture health before releases.

## Post-Publish Checks

Run after publishing completes:

```bash
npm view repopilot version
cargo search repopilot
gh release list --repo MykytaStel/repopilot --limit 5
```

Expected public version: `0.12.0`.
