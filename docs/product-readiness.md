# Product Readiness

RepoPilot is moving from an experimental CLI into a product that should be safe to install, easy to understand, stable in CI, and useful after the first five minutes.

This document defines the release gates for product quality.

RepoPilot's product promise:

```text
local-first repository audit
→ evidence-based findings
→ diff-aware review
→ baseline adoption
→ AI-ready remediation context
→ stable CI integration
```

RepoPilot should not become a random collection of commands or rules. Every release should improve at least one of these product qualities:

- lower false-positive noise;
- faster scans;
- clearer findings;
- more stable CLI behavior;
- better first-run onboarding;
- safer CI and release behavior;
- stronger local-first trust.

---

## Product readiness smoke suite

`scripts/smoke-product.sh` is the single install-like release smoke gate. Run it
from the repository root:

```bash
./scripts/smoke-product.sh
```

Run it against an already built binary:

```bash
./scripts/smoke-product.sh --binary ./target/release/repopilot
```

Run it against a different repository:

```bash
./scripts/smoke-product.sh --repo /path/to/project
```

Keep generated smoke artifacts:

```bash
./scripts/smoke-product.sh --keep-tmp
```

The script checks the main product workflows:

```text
version
help
init
doctor
scan console
scan JSON + receipt
scan Markdown
scan SARIF
baseline create
scan --baseline --fail-on new-high
review Markdown
ai context
ai plan
ai prompt
inspect knowledge
inspect eval-rules
inspect explain
inspect feedback
self-scan signal quality
```

`review` is skipped when the target path is not inside a Git repository.
The self-scan signal-quality gate fails on finding contract violations or
default-visible low-quality noisy findings.

---

## CLI stability gates

Before a release, these commands must work:

```bash
repopilot --help
repopilot --version
repopilot init --path /tmp/repopilot.toml
repopilot doctor .
repopilot scan .
repopilot scan . --format json --output /tmp/repopilot-scan.json
repopilot scan . --format json --output /tmp/repopilot-scan.json --receipt /tmp/repopilot-receipt.json
repopilot scan . --format markdown --output /tmp/repopilot-scan.md
repopilot scan . --format sarif --output /tmp/repopilot.sarif
repopilot baseline create . --output .repopilot/baseline.json
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
repopilot review .
repopilot ai context .
repopilot ai plan .
repopilot ai prompt .
repopilot inspect eval-rules --format json
repopilot inspect knowledge
repopilot inspect explain src/main.rs
repopilot inspect feedback .
```

The primary help surface should prefer the stable command shape:

```text
repopilot scan
repopilot review
repopilot baseline
repopilot compare
repopilot doctor
repopilot ai ...
repopilot inspect ...
```

The executable CLI surface should stay on the stable command families. Use
`ai ...` for AI-ready local Markdown and `inspect ...` for diagnostics.

---

## Exit code contract

RepoPilot's exit codes are part of the product contract:

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Findings exceeded the configured threshold |
| `2` | Invalid CLI/config/user input |
| `3` | Runtime or environment failure |

CI users should be able to rely on these codes.

Examples:

```bash
repopilot scan . --fail-on new-high
```

Expected behavior:

- exit `0` when there is no threshold breach;
- exit `1` when new high or critical findings are present;
- exit `2` for invalid CLI input;
- exit `3` for runtime/environment failures.

---

## Local-first trust gates

RepoPilot runtime commands must stay local-first:

- scans read files from disk;
- reports write to stdout or explicit output paths;
- scan receipts write only to explicit local paths;
- local feedback is read from `.repopilot/feedback.yml`, validated locally, and
  reported in `local_feedback` metadata when applied;
- no source code is uploaded;
- no telemetry is sent;
- AI commands do not call AI providers;
- `ai context`, `ai plan`, and `ai prompt` only format local scan output as Markdown.

The npm package must not download binaries during installation. It should install
the matching platform-specific optional package from npm, and the generated
platform packages must be built only from checksum-verified GitHub Release
archives.

```bash
npm install -g repopilot
REPOPILOT_BINARY_PATH=/path/to/repopilot repopilot --version
```

`REPOPILOT_BINARY_PATH` remains the fallback for environments that omit optional
dependencies or require a user-managed binary.

---

## CI readiness gates

Before release, verify:

- `cargo fmt --all -- --check` passes;
- `cargo clippy --all-targets --all-features -- -D warnings` passes;
- `cargo test --all` passes;
- `cargo package --list` looks correct;
- `cargo publish --dry-run` passes;
- `npm run test:npm` passes;
- `npm pack --dry-run` looks correct;
- platform npm packages can be generated from release archives and packed with no lifecycle scripts;
- RepoPilot's own repository has `.repopilotignore`, `.repopilot/baseline.json`,
  and a CI gate using `--baseline .repopilot/baseline.json --fail-on new-high`;
- `.repopilot/baseline.json` updates are reviewed as accepted existing debt and
  are not used just to make CI green;
- `repopilot inspect eval-rules --format json` reports zero missing findings,
  unexpected findings, contract violations, stable-id failures, and
  quality-gate failures;
- `scripts/check-signal-quality.py --scan-json <self-scan.json>` passes without
  `--warn-only` for the default RepoPilot self-scan;
- `repopilot doctor .` reports no adoption warnings for the repository;
- `scripts/smoke-product.sh` passes against the release binary.

The GitHub Action should support:

```yaml
with:
  command: scan
  receipt: repopilot-receipt.json
```

```yaml
with:
  command: review
```

```yaml
with:
  command: ai-context
```

Receipt output is scan-only. The action should fail clearly when `receipt` is
provided with `review`, `compare`, or AI commands.

The action should also write a concise `$GITHUB_STEP_SUMMARY` entry for `scan`,
`review`, `doctor`, `compare`, and AI commands so pull request reviewers can see
the RepoPilot result without opening artifacts first.

---

## What should not block a release

Do not block a release only because RepoPilot does not yet have more rules.

Rules are useful only when they produce low-noise, evidence-backed findings.

For a product-quality release, prefer:

```text
stable CLI
fast scan
clear docs
predictable CI
low false positives
safe distribution
```

over:

```text
more commands
more rules
more output formats
more internal abstractions
```
