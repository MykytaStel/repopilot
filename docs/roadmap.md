# RepoPilot Roadmap

This roadmap describes the pre-1.0 line. The core product bet is a fast,
local-first audit/review tool for maintainers who do not want SaaS, telemetry,
or source upload.

## Product Direction

RepoPilot should stay on this path:

```text
local scan -> evidence-backed findings -> risk-ranked review -> baseline adoption -> AI-ready local context -> CI gate
```

AI commands remain a local formatting layer. They should package scan evidence
for Claude Code, ChatGPT, Cursor, or another assistant, not become the primary
product dependency.

## Aggressive Product Cut

Do not add new rule families before 0.20.0 unless the existing families have:

- true-positive and false-positive fixtures;
- false-positive coverage for default-visible behavior;
- complete metadata and false-positive notes;
- docs for high/critical findings;
- a documented visibility policy;
- clean self-audit behavior;
- clean `repopilot inspect eval-rules --format json` output.

Default scans should show only high-trust/high-priority findings. Broad
heuristics such as long functions, complex files, TODO/FIXME/HACK markers, and
testing gaps belong in `--profile strict` unless they become contextually precise
enough to earn default visibility.

## 0.13.x: Smart Baseline

0.13.x is a trust and adoption release line.

Focus:

- lock the audit-first positioning;
- keep the stable top-level surface to `scan`, `review`, `baseline`, `compare`,
  `doctor`, `inspect`, `ai`, `init`, and `cache`;
- harden `src/scan/scanner/mod.rs::finalize_report` and the scan finalization
  path;
- expand `inspect eval-rules` fixtures for default-visible and stable rules;
- reduce README/docs IA to install, first five minutes, core promise, and links;
- archive old GTM and release announcement/checklist docs.

Core rule families to strengthen before expanding:

- secrets and private keys;
- Rust panic/runtime risk;
- JavaScript, Python, Go, JVM, and .NET runtime exits;
- import graph risk;
- review diff and blast-radius behavior.

## Release Train

| Version | Theme | Main outcome |
|---|---|---|
| 0.13.x | Smart Baseline | Trustable default scan, fixture-backed stable rules, slimmer docs, baseline/review adoption. |
| 0.14 | Rule-author workflow | Broader fixture coverage, false-positive suites, and clearer rule decision debugging. |
| 0.15 | Adoption hardening | Workspace noise reduction, baseline ergonomics, and performance budgets for larger repositories. |
| 0.16 | Distribution trust | Release artifact attestations and tighter npm, crates.io, Homebrew, and installer verification. |
| 0.17 | Curated packs | First-party rule/knowledge packs only if the lifecycle gate stays healthy. |
| 0.18 | Compatibility docs | Migration, support, and schema compatibility policy for v1. |
| 0.19 | v1 cleanup prep | Final deprecations, alias policy, and schema migration notes. |
| 0.20 | v1 candidate review | Confirm the exact 1.0.0 scope and block unproven expansion. |

## Test Plan

Keep the test pyramid:

- unit tests for pure logic;
- fixture tests for rule precision and false positives;
- schema/golden tests for JSON, SARIF, and reports;
- minimal CLI smoke tests for command paths.

Do not mechanically reduce the current test count. Remove duplicates only when
they exercise the same renderer or CLI path without covering a new risk, and
prefer table-driven cases for repeated rule/renderer checks.

Required release gates:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm run test:npm
./scripts/smoke-product.sh
repopilot inspect eval-rules --format json
repopilot scan .
```

## 1.0.0 Gates

RepoPilot can move to 1.0.0 only after the 0.20 review confirms:

- the stable top-level command surface is limited to `scan`, `review`,
  `baseline`, `compare`, `doctor`, `inspect`, `ai`, `init`, and `cache`;
- JSON, SARIF, baseline, receipt, report-envelope, and diagnostics schemas have
  compatibility rules;
- local feedback metadata is visible so suppressions never silently hide risk;
- the local-first trust model has no telemetry, source upload, hosted scanner,
  or implicit LLM API calls;
- release verification and product smoke suites pass from a clean release branch;
- self-audit produces no P0/P1 findings and no high/critical findings by default;
- distribution channels are verified for npm, crates.io, GitHub Releases,
  Homebrew, and curl install.

1.0.0 should require a trustworthy product contract, not every possible rule
family. More rules can ship after v1 only if the lifecycle, evidence, docs, and
tests remain disciplined.
