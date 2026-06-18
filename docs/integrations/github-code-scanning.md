# GitHub Pull Request Integration

RepoPilot 0.16 can produce a job summary, capped workflow annotations, JSON and
SARIF artifacts, typed outputs, and an optional sticky PR comment from one
review run.

## Reusable Workflow

Copy `.github/workflows/repopilot-pr-review.yml` into the repository that hosts
the reusable workflow, then call it from a PR workflow:

```yaml
name: RepoPilot

on:
  pull_request:

permissions:
  contents: read

jobs:
  review:
    uses: MykytaStel/repopilot/.github/workflows/repopilot-pr-review.yml@v0.18.0
    with:
      fail-on-review: definitely
      fail-on-priority: p1
```

The workflow checks out full history, derives base/head SHAs from the PR event,
runs one changed-scope JSON review, writes secondary SARIF, emits at most 20
detailed annotations, and uploads the JSON/SARIF/Markdown artifacts.

This default is read-only and works for fork PRs. It does not use
`pull_request_target`.

## First-Party Action

```yaml
- uses: actions/checkout@v6
  with:
    fetch-depth: 0

- name: RepoPilot review
  id: repopilot
  uses: MykytaStel/repopilot@v0.18.0
  with:
    command: review
    scope: changed
    profile: default
    fail-on-review: definitely
    fail-on-priority: p1
```

The Action checksum-verifies and caches the exact release binary by
version/OS/architecture. It exposes:

- `conclusion`, `exit-code`, and `gate-result`;
- `findings-count` and `signals-count`;
- `review-json-file`, `review-sarif-file`, and `sarif-file`.

## SARIF Upload

SARIF upload is opt-in because `security-events: write` is not available to all
fork PR contexts:

```yaml
permissions:
  contents: read
  security-events: write

steps:
  - uses: MykytaStel/repopilot@v0.18.0
    with:
      command: review
      upload-sarif: "true"
```

Review SARIF contains in-diff scan findings and concrete taint issues.
Boundary and algorithmic facts remain workflow annotations rather than Code
Scanning alerts.

Generate the same artifacts locally:

```bash
repopilot review . --base origin/main \
  --format json --output repopilot-review.json \
  --sarif-output repopilot-review.sarif
```

## Sticky Comment

Comments are opt-in:

```yaml
permissions:
  contents: read
  pull-requests: write

steps:
  - uses: MykytaStel/repopilot@v0.18.0
    with:
      command: review
      comment: "true"
```

Use comment mode only where `pull-requests: write` is intentionally granted.
The default job summary and artifacts do not need it.

## Full Scan

Repository-wide SARIF remains available:

```bash
repopilot scan . --format sarif --output repopilot.sarif
```

Use `scan` for scheduled/full audits and changed-scope `review` for PR feedback.
