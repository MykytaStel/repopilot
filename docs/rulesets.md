# Audit Rulesets

RepoPilot findings are identified by a stable `rule_id`. Each rule belongs to a category and carries a fixed severity.

## Categories

| Category | Serialized value | Description |
|---|---|---|
| Architecture | `ARCHITECTURE` | Structural or design concerns |
| Code quality | `CODE_QUALITY` | Hygiene and maintainability issues |
| Testing | `TESTING` | Test coverage or quality gaps |
| Security | `SECURITY` | Potential vulnerabilities |
| Performance | `PERFORMANCE` | Performance-related concerns |

## Severity levels

| Level | Serialized value |
|---|---|
| Info | `INFO` |
| Low | `LOW` |
| Medium | `MEDIUM` |
| High | `HIGH` |
| Critical | `CRITICAL` |

## Rules

### `code-marker.todo`

| Field | Value |
|---|---|
| Category | `CODE_QUALITY` |
| Severity | `LOW` |
| Description | A `TODO` comment was found. Review whether it represents outstanding work that should be tracked. |

### `code-marker.fixme`

| Field | Value |
|---|---|
| Category | `CODE_QUALITY` |
| Severity | `MEDIUM` |
| Description | A `FIXME` comment was found. These typically indicate known bugs or broken behavior. |

### `code-marker.hack`

| Field | Value |
|---|---|
| Category | `CODE_QUALITY` |
| Severity | `MEDIUM` |
| Description | A `HACK` comment was found. These indicate workarounds that should be replaced with proper solutions. |

### `architecture.large-file`

| Field | Value |
|---|---|
| Category | `ARCHITECTURE` |
| Severity | `MEDIUM` when LOC ≥ threshold; `HIGH` when LOC ≥ huge threshold |
| Default threshold | 300 non-empty LOC |
| Override | `--max-file-loc <n>` |
| Description | A file exceeds the configured LOC threshold. Consider splitting responsibilities into smaller modules. |

The *huge* threshold defaults to `max(threshold × 3, threshold + 1)`. When `--max-file-loc` is set, the huge threshold adjusts automatically.

## Evidence

Every finding includes structured evidence:

| Field | Type | Description |
|---|---|---|
| `path` | path | File that triggered the finding |
| `line_start` | usize | First relevant line (1-based) |
| `line_end` | usize? | Last relevant line (optional) |
| `snippet` | string | Source text at that location |
