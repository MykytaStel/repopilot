# RepoPilot 0.12 GTM Plan

## Positioning

RepoPilot is the local-first repository safety layer for teams using AI-assisted
development and baseline-aware CI. It does not replace language linters. It adds
cross-repository risk signals, PR review context, accepted-debt baselines, and
AI-ready remediation context without source upload or telemetry.

Primary promise:

```text
Find the repo-level risks worth fixing next, keep existing debt from blocking CI,
and give AI tools focused local context.
```

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

Buyer/user pain:

- AI edits can miss architecture, security, and test context outside the active
  file;
- broad scan reports are too noisy to paste into an assistant;
- hosted tools are not acceptable for private source code.

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

CTA:

```bash
repopilot scan .
repopilot ai context . --budget 4k
```

## CI Teams

Message:

```text
Adopt gradually with baseline-aware gates, SARIF, receipts, and no source upload.
```

Buyer/user pain:

- teams want a gate for new risk but cannot stop every PR over old debt;
- reviewers need report evidence in the PR, not only a failing status check;
- security and release teams need local artifacts they can archive.

Primary commands:

```bash
repopilot baseline create . --output .repopilot/baseline.json
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
repopilot review . --base origin/main --baseline .repopilot/baseline.json --fail-on-priority p1
```

Proof points:

- GitHub Action job summaries for PR readability;
- SARIF upload for Code Scanning;
- receipts for release evidence;
- local feedback metadata so suppressions are auditable.

CTA:

```bash
repopilot doctor .
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
```

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

## Channels And Assets

- README: install, quick start, baseline adoption, AI workflow.
- Release announcement: trust release story plus one copy-paste CI example.
- GitHub Marketplace/action docs: scan, review, receipt, and SARIF use cases.
- Short posts:
  - AI workflow: "local repo scan -> focused context -> assistant patch";
  - CI workflow: "baseline existing debt, fail only new high risk";
  - React Native workflow: "Hermes/New Architecture/deprecated API checks before release".

## Qualification

Best early users:

- teams already using Claude Code, ChatGPT, Cursor, or similar tools on private
  repositories;
- teams that want CI gating but have existing architecture or testing debt;
- React Native and Expo teams preparing releases.

Not the right pitch:

- "replace ESLint/Clippy/Ruff";
- "hosted dashboard";
- "automatic AI fixer";
- "telemetry-driven scanner".
