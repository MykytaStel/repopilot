# RepoPilot 0.13.0 Release Announcement

RepoPilot 0.13 is the trust and usability release: cleaner default output,
stronger rule quality checks, safer baseline adoption, and better evidence for
CI and AI-assisted remediation.

## Suggested Post

RepoPilot 0.13.0 is ready.

This release is about making repository audits easier to trust and easier to
adopt:

- cleaner default scans that keep broad maintainability and testing noise out of
  the normal CI path;
- stronger rule quality checks through `inspect eval-rules`, fixture coverage,
  stable finding IDs, and finding contract validation;
- safer adoption with `.repopilot/baseline.json` and
  `scan --baseline --fail-on new-high`;
- better evidence for teams: compact console output, full diagnostics on demand,
  JSON, SARIF, Markdown, receipts, and signal-quality summaries;
- local AI remediation context that uses the same product scan path and default
  visibility policy as normal scans.

Try it:

```bash
repopilot scan .
repopilot baseline create . --output .repopilot/baseline.json
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
repopilot ai context . --budget 4k
repopilot inspect eval-rules --format json
```

CI evidence:

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot scan . --format json --output repopilot-report.json --receipt .repopilot/receipt.json
repopilot scan . --format sarif --output repopilot.sarif
```

GitHub Actions:

```yaml
- uses: MykytaStel/repopilot@v0.13.0
  with:
    command: scan
    baseline: .repopilot/baseline.json
    fail-on: new-high
    receipt: repopilot-receipt.json
```

Positioning:

```text
RepoPilot 0.13 is the trust and usability release:
cleaner default output, stronger rule quality checks, safer baseline adoption,
and better evidence for CI and AI-assisted remediation.
```

## Post-Publish Checks

Run after publishing completes:

```bash
npm view repopilot version
cargo search repopilot
gh release list --repo MykytaStel/repopilot --limit 5
```

Expected public version: `0.13.0`.
