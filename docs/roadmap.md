# RepoPilot Roadmap

This roadmap describes the pre-1.0 release line starting with RepoPilot 0.12.0. It is
directional, but the v1 gates are release criteria: RepoPilot should not ship
1.0.0 until those criteria are met.

## Product Direction

RepoPilot stays a local-first repository audit CLI:

```text
local scan -> evidence-backed findings -> risk-ranked review -> baseline adoption -> AI-ready local context -> CI gate
```

The product should improve signal quality, scan reliability, rule lifecycle
discipline, and distribution trust before adding broad custom execution surfaces.

## Release Train

| Version | Theme | Main outcome |
|---|---|---|
| 0.12 | Core foundation | Document the rule lifecycle, v1 gates, and local-first learning policy; refactor core scan and Knowledge Engine boundaries without public schema changes. |
| 0.13 | Local overlays MVP | Prototype local project/user knowledge overlays for calibration only, behind an explicit unstable surface. |
| 0.14 | Rule-author workflow | Add validation and inspection workflows for rule authors, false-positive fixtures, and clearer decision debugging. |
| 0.15 | Adoption hardening | Improve workspace noise, baseline ergonomics, and performance budgets for larger repositories. |
| 0.16 | Distribution trust | Add release artifact attestations and tighten npm, crates.io, Homebrew, and installer verification. |
| 0.17 | Curated packs | Introduce curated first-party knowledge/rule pack structure if the 0.13 overlay model is stable. |
| 0.18 | Compatibility docs | Document migration, compatibility, and support policy for the stable v1 command surface. |
| 0.19 | v1 cleanup prep | Finalize deprecations, legacy alias policy, and any schema migration notes. |
| 0.20 | v1 candidate review | Run the v1 gate review and decide the exact 1.0.0 scope. |

## Local-First Learning Policy

RepoPilot does not train a model, upload source code, or send telemetry. In this
roadmap, "learning" means local, inspectable artifacts that a user can review,
commit, diff, or delete.

Allowed directions:

- local calibration overlays that adjust severity, confidence, or risk weights;
- local false-positive fixtures that prove a rule should be narrowed;
- project-specific context hints committed by the team;
- explicit user commands that explain why a rule applied or was suppressed.

Not allowed for the 0.x line:

- hosted source-code analysis;
- hidden telemetry;
- automatic remote model training;
- arbitrary plugin code execution during scans;
- silently changing rule behavior based on unreviewed local state.

## 1.0.0 Gates

RepoPilot can move to 1.0.0 only after the 0.20 review confirms:

- stable primary command surface: `scan`, `review`, `baseline`, `compare`, `doctor`, `ai`, and `inspect`;
- documented policy for legacy 0.x aliases;
- stable JSON, SARIF, baseline, and receipt schema compatibility rules;
- documented Knowledge Engine lifecycle from audit registration to risk calibration;
- local-first trust model with no telemetry, source upload, hosted scanner, or LLM API calls;
- release verification and product smoke suites pass from a clean release branch;
- self-audit produces no P0/P1 findings and no high/critical findings by default;
- distribution channels are verified for npm, crates.io, GitHub Releases, Homebrew, and curl install.

## What Should Not Block 1.0.0

1.0.0 should not require every possible rule family. It should require a stable,
trustworthy product contract. More rules can ship after v1 if the rule lifecycle,
evidence, docs, and tests remain disciplined.
