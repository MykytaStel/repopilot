# RepoPilot Scan Report

## Overview

- **RepoPilot version:** 0.11.0
- **Path:** `polyglot-api`
- **Risk:** Moderate
- **Health score:** 88/100
- **Findings:** 5 (1.6/kloc)
- **Files analyzed:** 18
- **Directories analyzed:** 6
- **Lines of code:** 3,120

## Risk Summary

- **Severity:** 1 high, 3 medium, 1 low
- **Categories:** security 1, architecture 1, code-quality 2, testing 1
- **Top paths:** `src/config.py` 1, `server/main.go` 1, `web/api/client.ts` 1

## Top Risk Clusters

| Priority | Area | Rule | Count | Max severity | Max risk | Examples |
| --- | --- | --- | ---: | --- | ---: | --- |
| P1 | `src` | `security.secret-candidate` | 1 | HIGH | 97 | `src/config.py:1` |
| P2 | `server` | `language.go.panic-exit-risk` | 1 | MEDIUM | 60 | `server/main.go:18` |
| P2 | `web/api` | `architecture.deep-relative-imports` | 1 | MEDIUM | 50 | `web/api/client.ts:4` |
| P3 | `server` | `code-marker.todo` | 1 | LOW | 26 | `server/main.go:3` |

## Findings

### Security

#### `security.secret-candidate` (1)

- **[HIGH] Possible secret detected**
  - Confidence: MEDIUM
  - Priority: P1 (risk 97/100)
  - Risk signals: security finding (+12), production code (+10)
  - Location: `src/config.py:1`
  - Evidence: `src/config.py:1` - API_KEY = "sk_live_..."
  - Recommendation: Move the value to an environment variable or secrets manager and rotate the exposed credential.

### Code Quality

#### `language.go.panic-exit-risk` (1)

- **[MEDIUM] Risky Go log.Fatal usage**
  - Confidence: MEDIUM
  - Priority: P2 (risk 60/100)
  - Location: `server/main.go:18`
  - Recommendation: Return errors from reusable code and centralize process exits at the CLI boundary.

#### `code-marker.todo` (1)

- **[LOW] TODO marker found**
  - Confidence: MEDIUM
  - Priority: P3 (risk 26/100)
  - Risk signals: backlog marker (-4)
  - Location: `server/main.go:3`
  - Recommendation: Convert the TODO into a tracked issue or resolve it.

### Architecture

#### `architecture.deep-relative-imports` (1)

- **[MEDIUM] Deep relative import**
  - Confidence: MEDIUM
  - Priority: P2 (risk 50/100)
  - Location: `web/api/client.ts:4`
  - Recommendation: Introduce a stable module boundary or path alias before the import chain spreads.
