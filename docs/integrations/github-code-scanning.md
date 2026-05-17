# GitHub Code Scanning

RepoPilot can write SARIF so GitHub Code Scanning can display findings in the repository security tab and annotate pull requests.

## Generate SARIF locally

```bash
repopilot scan . --format sarif --output repopilot.sarif
```

If your installed RepoPilot version does not support `--output`, redirect stdout instead:

```bash
repopilot scan . --format sarif > repopilot.sarif
```

When validating changes from a RepoPilot source checkout, use the local binary:

```bash
cargo run -- scan . --format sarif --output repopilot.sarif
```

## GitHub Actions

Copy [`.github/workflows/repopilot-sarif.yml.example`](../../.github/workflows/repopilot-sarif.yml.example) to `.github/workflows/repopilot-sarif.yml` in a repository where you want RepoPilot findings uploaded to GitHub Code Scanning.
For baseline-aware gating plus SARIF upload, copy [`.github/workflows/repopilot-baseline-sarif.yml.example`](../../.github/workflows/repopilot-baseline-sarif.yml.example).

```yaml
name: RepoPilot SARIF

on:
  pull_request:
  push:
    branches:
      - main

permissions:
  contents: read
  security-events: write

jobs:
  repopilot:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v5

      - name: Install RepoPilot
        run: cargo install repopilot

      - name: Run RepoPilot SARIF scan
        run: repopilot scan . --format sarif --output repopilot.sarif

      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v4
        with:
          sarif_file: repopilot.sarif
          category: repopilot
```

The workflow needs `security-events: write`; without it, GitHub rejects the SARIF upload. `contents: read` is enough for checkout.

For RepoPilot's own repository or another Rust project where installing from crates.io is too slow during CI, run the local checkout binary instead:

```yaml
      - name: Run RepoPilot SARIF scan
        run: cargo run -- scan . --format sarif --output repopilot.sarif
```

Keep the SARIF file name consistent between the scan step and the upload step. The examples above write and upload `repopilot.sarif`.

## First-party action with receipt artifact

The first-party action can generate SARIF and a compact audit receipt from the
same scan. `receipt` is only valid with `command: scan`.

```yaml
name: RepoPilot

on:
  pull_request:

permissions:
  contents: read
  security-events: write

jobs:
  repopilot:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v5

      - name: Run RepoPilot
        id: repopilot
        uses: MykytaStel/repopilot@v0.12.0
        with:
          command: scan
          baseline: .repopilot/baseline.json
          fail-on: new-high
          min-priority: p2
          receipt: repopilot-receipt.json

      - name: Upload audit receipt
        uses: actions/upload-artifact@v4
        if: always() && steps.repopilot.outputs.receipt-file != ''
        with:
          name: repopilot-receipt
          path: ${{ steps.repopilot.outputs.receipt-file }}
```

For review-only gates, use `command: review` without `receipt`:

```yaml
      - name: RepoPilot review gate
        uses: MykytaStel/repopilot@v0.12.0
        with:
          command: review
          base: origin/main
          baseline: .repopilot/baseline.json
          fail-on-priority: p1
          upload-sarif: "false"
```

For adoption diagnostics or focused rule investigations, use the same first-party
action without SARIF upload:

```yaml
      - name: RepoPilot doctor
        uses: MykytaStel/repopilot@v0.12.0
        with:
          command: doctor
          format: markdown
          output: repopilot-doctor.md
          upload-sarif: "false"

      - name: RepoPilot focused scan
        uses: MykytaStel/repopilot@v0.12.0
        with:
          command: scan
          rule: language.rust.panic-risk
          min-priority: p2
          timing: "true"
          upload-sarif: "false"
```

## Choosing an output format

Use JSON when a script or service will parse RepoPilot results directly.

Use Markdown when you want a human-readable report for pull request comments, job summaries, or issue descriptions.

Use SARIF when you want CI and code scanning integrations, including GitHub Code Scanning.
