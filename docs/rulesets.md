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

Markdown files and files under `tests/` or `test/` are skipped to avoid reporting documentation examples and test fixture strings as production debt.

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

### `architecture.too-many-modules`

| Field | Value |
|---|---|
| Category | `ARCHITECTURE` |
| Severity | `MEDIUM` |
| Default threshold | 20 files per directory |
| Override | `--max-directory-modules <n>` |
| Description | A directory contains more files than the threshold. Consider splitting it into sub-modules to reduce coupling. |

### `architecture.deep-nesting`

| Field | Value |
|---|---|
| Category | `ARCHITECTURE` |
| Severity | `LOW` |
| Default threshold | 5 directory levels below the scan root |
| Override | `--max-directory-depth <n>` |
| Description | The deepest file in the project exceeds the nesting threshold. Deep hierarchies often indicate over-engineered module structures. |

### `testing.missing-test-folder`

| Field | Value |
|---|---|
| Category | `TESTING` |
| Severity | `MEDIUM` |
| Description | No test directory (`tests/`, `test/`, `__tests__/`, `spec/`) was found at any level in the scanned tree. |

### `testing.source-without-test`

| Field | Value |
|---|---|
| Category | `TESTING` |
| Severity | `LOW` |
| Description | A source file has no detectable test counterpart. Looks for sibling files like `payment_test.rs`, `payment.test.ts`, `payment.spec.ts`, entries in a `tests/` directory, or Rust integration tests such as `tests/report_writer.rs` for `src/report/writer.rs`. |

Low-signal wrapper entrypoints such as `mod.rs`, `lib.rs`, and `main.rs` are skipped.

### `security.secret-candidate`

| Field | Value |
|---|---|
| Category | `SECURITY` |
| Severity | `HIGH` |
| Description | A line matches a pattern indicating a hardcoded secret (`api_key`, `password`, `token`, `private_key`, etc.). The evidence snippet **masks the value** — only the key name and first 3 characters are shown. |

Skipped for files whose path contains `test`, `fixture`, `example`, or `mock`.

### `security.private-key-candidate`

| Field | Value |
|---|---|
| Category | `SECURITY` |
| Severity | `CRITICAL` |
| Description | A PEM private key header (`-----BEGIN RSA PRIVATE KEY-----`, etc.) was found in a source file. The key content is never included in the evidence. |

Markdown examples under `docs/` are skipped to avoid reporting documented rule examples as committed keys.

### `security.env-file-committed`

| Field | Value |
|---|---|
| Category | `SECURITY` |
| Severity | `HIGH` |
| Description | A `.env`, `.env.local`, `.env.production`, `.env.staging`, or `.env.development` file is present in the scanned tree. Environment files often contain secrets and must not be committed. |

## Evidence

Every finding includes structured evidence:

| Field | Type | Description |
|---|---|---|
| `path` | path | File that triggered the finding |
| `line_start` | usize | First relevant line (1-based) |
| `line_end` | usize? | Last relevant line (optional) |
| `snippet` | string | Source text at that location |
