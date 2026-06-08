# Configuration

RepoPilot can run with built-in defaults, CLI overrides, and a project-level
`repopilot.toml`.

## Create a config

```bash
repopilot init
```

Overwrite an existing file:

```bash
repopilot init --force
```

Write to a custom path:

```bash
repopilot init --path ./config/repopilot.toml
```

## Precedence

Configuration is resolved in this order:

```text
CLI arguments > repopilot.toml > built-in defaults
```

Use CLI flags for one-off local experiments. Use `repopilot.toml` for team-wide
settings that should be committed.

## Example

```toml
[scan]
ignore = [
  ".git",
  ".repopilot",
  "target",
  "node_modules",
  "dist",
  "build",
  ".next",
  ".nuxt",
  ".cache",
  "coverage",
  "vendor",
  "Pods",
  "DerivedData"
]
max_file_bytes = 2097152

[review]
scope = "changed"
fail_on = "none"

[architecture]
max_file_lines = 300
huge_file_lines = 1000
max_directory_modules = 20
max_directory_depth = 5
max_function_lines = 50
max_fan_out = 15
instability_hub_min_fan_in = 5
instability_hub_min_instability_pct = 75

[testing]
detect_missing_tests = true

[security]
detect_secret_like_names = true

[output]
default_format = "console"
```

`[review] scope` accepts `changed` or `full`. `[review] fail_on` accepts `none`
or `definitely`. CLI flags have higher priority:

```bash
repopilot review . --scope full --profile strict
repopilot review . --fail-on-review definitely
```

## Common CLI overrides

```bash
repopilot scan . --max-file-loc 500
repopilot scan . --max-directory-modules 30
repopilot scan . --max-directory-depth 8
repopilot scan . --max-file-size 1mb
repopilot scan . --max-files 1000
repopilot scan . --exclude generated
repopilot scan . --include-low-signal
```

## Presets

Use presets when you want fast tuning without editing config:

```bash
repopilot scan . --preset strict
repopilot scan . --preset balanced
repopilot scan . --preset lenient
```

Suggested use:

| Preset | Best for |
|---|---|
| `strict` | new projects and green-field code |
| `balanced` | default project checks |
| `lenient` | legacy repositories adopting RepoPilot gradually |

## Ignore files

RepoPilot respects `.gitignore` and supports `.repopilotignore` for audit-specific
exclusions.

Use `.repopilotignore` for files that should stay in the repository but not affect
audit quality:

```text
fixtures/
snapshots/
generated/
vendor/
```

## Baseline adoption

For legacy repositories, create a baseline before enforcing CI gates:

```bash
repopilot baseline create .
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
```

Do not refresh the baseline just to silence CI. Refresh it only when the team
explicitly accepts the current findings as technical debt.

## Local Feedback

RepoPilot reads repository-local suppressions from `.repopilot/feedback.yml`.
Finding suppressions use `rule_id + path`; review-signal suppressions use
namespaced `kind + path`.

```yaml
suppressions:
  - rule_id: architecture.large-file
    path: "src/generated/schema.rs"
    reason: generated schema boundary
  - kind: behavioral.network-call-added
    path: "src/generated/**"
    reason: generated client transport
    expires: "2026-12-31"
```

Suppressions are applied by `scan` and `review`; malformed entries surface as
report diagnostics rather than silently dropping findings.

Use `--ignore-feedback` on `scan` or `review` for an unsuppressed report.
Expired review suppressions no longer apply. Reports expose suppression counts
through `local_feedback` metadata so policy never hides findings silently.

Do not commit `.repopilot/feedback.yml` by default. Commit it only when the
suppression is a reviewed team policy; keep temporary or personal suppressions
local.
