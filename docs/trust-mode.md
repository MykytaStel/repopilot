# Trust Mode

RepoPilot's default scan output is intentionally quieter than a raw audit dump.

The goal of the default profile is to answer:

- what can break production
- what can leak secrets
- what should block a release
- what is safe to review later
- what should move into a strict/deep-audit workflow

## Profiles

```bash
repopilot scan .
repopilot scan . --profile default
repopilot scan . --profile strict
repopilot scan . --include-maintainability
```

`default` is optimized for day-to-day development and CI review. It hides broad
maintainability and testing heuristics by policy, including long-function,
complex-file, TODO/FIXME/HACK, and testing-gap rules.

`strict` preserves the full raw audit output. Use it for refactoring passes,
codebase cleanup, rule development, and release-hardening audits.

## Visibility intent model

RepoPilot does not decide visibility only from severity. A high-severity large
file can still be a maintainability concern, while a medium/high-confidence
runtime signal can be important if it affects production behavior.

Every finding is classified into a product-level intent:

```text
SecurityRisk
RuntimeRisk
ActionableRisk
Maintainability
TestingGap
Informational
```

The visibility layer then applies profile policy:

```text
raw finding
  -> intent classification
  -> default/strict policy
  -> visible finding or hidden suggestion
```

This keeps audit rules focused on evidence and keeps report policy centralized.

## Hidden suggestion summaries

Default profile does not simply drop hidden findings. It records a structured
breakdown in the scan summary:

```json
{
  "hidden_suggestions": [
    {
      "intent": "maintainability",
      "rule_id": "architecture.large-file",
      "category": "architecture",
      "reason": "maintainability signals are hidden in the default profile",
      "count": 43
    }
  ]
}
```

Console and Markdown reports show the top hidden groups so users understand what
was hidden and why. This makes the default report quieter without making it
opaque.

## Why this matters

Earlier versions of RepoPilot could produce technically valid but noisy reports.
For example, missing-test suggestions, long functions, large files, and script
`process.exit` calls could dominate the default report even when they were not
release-blocking risks.

Trust Mode makes the default report more useful:

```text
default profile = actionable production/security/runtime risks
strict profile  = full maintainability/testing/deep audit output
```

## Examples

A real secret candidate stays visible by default:

```text
security.secret-candidate -> SecurityRisk -> visible
```

A missing source test is hidden by default:

```text
testing.source-without-test -> TestingGap -> hidden
```

A script-level process exit is hidden by default:

```text
scripts/verify-release.mjs process.exit(1) -> RuntimeRisk -> hidden
```

A reusable source-level process exit stays visible:

```text
src/runtime/server.ts process.exit(1) -> RuntimeRisk -> visible
```

A large file is hidden by default as a maintainability suggestion:

```text
architecture.large-file -> Maintainability -> hidden
```

A high-priority architecture/coupling issue can remain visible:

```text
architecture.circular-dependency -> ActionableRisk -> visible when high priority or high confidence
```

## Future improvements

The current intent model is still rule-id aware, but visibility policy is now
centralized and semantic. The next improvements should be:

- persisted hidden summaries in SARIF properties
- `repopilot eval` fixtures for visibility behavior
- validated local feedback with visible `local_feedback` metadata
- confidence calibration per rule
- project profile specific visibility defaults

## Path-aware architecture heuristics

Some repository paths are intentionally deep and should not be treated as production architecture
risk. Rule fixtures, test corpora, examples, docs, generated files, vendor trees, and build output
may use nested paths to describe scenarios rather than product structure.

For that reason, broad project-level architecture heuristics such as `architecture.deep-nesting`
are scoped to production architecture candidates before they emit findings. This keeps rule-author
fixtures useful for evaluation without turning them into user-facing architecture noise.

```text
tests/fixtures/rules/** -> fixture corpus -> not a production deep-nesting finding
src/features/**         -> production source -> eligible for deep-nesting analysis
```

## Architecture anti-pattern scope

Architecture anti-pattern rules are production-scoped by default. Broad structure heuristics such as
`architecture.deep-nesting` should not treat rule fixtures, test corpora, docs, examples, generated
files, vendor trees, or build output as product architecture risk.

This keeps default output quiet while preserving those paths for rule evaluation and other audits
where they are intentionally useful.
