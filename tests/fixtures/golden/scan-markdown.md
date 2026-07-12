# RepoPilot Scan Report

## Overview

- **RepoPilot version:** {{VERSION}}
- **Path:** `demo-project`
- **Profile:** `default`
- **Scope:** full scan; repo-level rules included
- **Risk:** Moderate
- **Health score:** 90/100
- **Findings:** 1 visible (23.8/kloc)
- **Hidden suggestions:** 2 strict-only suggestions
- **Note:** 2 strict-only suggestions hidden. Run with `--profile strict` or `--include-maintainability` to view.
- **Top hidden suggestions:**
  - `code-quality` / `maintainability` / `code-quality.long-function`: 2 - maintainability signals are hidden in the default profile
- **Files analyzed:** 2
- **Directories analyzed:** 2
- **Non-empty lines:** 42

## Risk Summary

- **Severity:** 1 high
- **Categories:** security (1)
- **Top paths:** src/config.ts (1)

## Signal Quality

- **High confidence:** 1
- **Medium confidence:** 0
- **Low confidence:** 0
- **Evidence coverage:** 100%
- **Recommendation coverage:** 100%
- **Contract warnings:** 0

## Top Risk Clusters

| Priority | Area | Rule | Count | Max severity | Max risk | Examples |
| --- | --- | --- | ---: | --- | ---: | --- |
| P0 | `src` | `security.secret-candidate` | 1 | HIGH | 95 | `src/config.ts:3` |

## Top Rules

| Rule | Count | Max severity |
| --- | ---: | --- |
| `security.secret-candidate` | 1 | HIGH |

## Languages

| Language | Files |
| --- | ---: |
| Rust | 1 |
| TypeScript | 1 |

## Findings Index

| Category | Rule | Max severity | Count | First location |
| --- | --- | --- | ---: | --- |
| security | `security.secret-candidate` | HIGH | 1 | src/config.ts:3 |

## Findings

### Security

#### `security.secret-candidate` (1)

- **[HIGH] Possible hardcoded secret or API key**
  - Confidence: HIGH
  - Priority: P0 (risk 95/100)
  - Risk signals: +75 severity: high severity finding, +8 confidence: high confidence signal, +12 security: security findings are prioritized
  - Location: `src/config.ts:3`
  - Evidence: `src/config.ts:3` - [sensitive evidence redacted]
  - Context: A high-entropy string or a pattern matching a known secret format was found in source code
  - Recommendation: Move the value to an environment variable or secrets manager
  - Verification:
    - Open src/config.ts:3 and confirm the flagged code shown is still present: `[sensitive evidence redacted]`.
    - Confirm the flagged input reaches this code without validation or sanitization, and check for an existing test covering this path.
    - This plan is generated from static evidence only — it does not execute code, run tests, or observe runtime behavior. Treat it as a starting point for manual or test-based confirmation, not proof the finding is a true or false positive.
  - Docs: https://github.com/MykytaStel/repopilot/blob/main/docs/security.md#secret-handling

