# RepoPilot 0.12 GTM Plan

RepoPilot's launch order for 0.12.0:

1. Publish `0.12.0` across GitHub Releases, crates.io, npm, and Homebrew tap.
2. Publish or update GitHub Marketplace/action docs.
3. Post the AI workflow story.
4. Post the CI/baseline story.
5. Post the React Native/Expo proof story.

## AI Developers

Message:

```text
Scan repo locally, paste AI-ready context into Claude Code, ChatGPT, or Cursor,
then fix with less surprise.
```

Primary commands:

```bash
repopilot ai context .
repopilot ai plan .
repopilot ai prompt .
```

Proof points:

- no telemetry, no source upload, no automatic LLM API calls;
- risk-ranked findings with evidence and recommendations;
- focused context budgets for security, architecture, quality, or framework work.

## CI Teams

Message:

```text
Adopt gradually with baseline-aware gates, SARIF, receipts, and no source upload.
```

Primary commands:

```bash
repopilot baseline create .
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
repopilot review . --base origin/main --baseline .repopilot/baseline.json --fail-on-priority p1
```

Proof points:

- GitHub Action job summaries for PR readability;
- SARIF upload for Code Scanning;
- receipts for release evidence;
- local feedback metadata so suppressions are auditable.

## React Native And Expo

Message:

```text
Catch React Native architecture health issues locally before a release.
```

Primary commands:

```bash
repopilot scan . --format markdown --output rn-architecture-health.md
repopilot ai context . --focus framework --budget 4k
```

Proof points:

- Hermes and New Architecture signal checks;
- deprecated React Native APIs;
- async storage/core import and direct state mutation checks;
- normal repository risks like large files, testing gaps, and secret candidates.

## Trust Angle

Use these claims only after the matching release workflows have completed:

- GitHub Release archives include checksums and artifact attestations;
- npm packages publish through Trusted Publishing / GitHub OIDC;
- npm install uses platform optional packages and no postinstall downloader;
- runtime commands stay local-first.
