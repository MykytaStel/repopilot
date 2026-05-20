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

Run the product smoke suite from the repository root:

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
scan JSON + receipt
scan Markdown
review Markdown
ai context
ai plan
ai prompt
inspect knowledge
inspect explain
inspect feedback
```

`review` is skipped when the target path is not inside a Git repository.

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
repopilot review .
repopilot ai context .
repopilot ai plan .
repopilot ai prompt .
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
