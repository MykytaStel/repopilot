# Audit Rulesets

RepoPilot findings are identified by a stable `rule_id`. RepoPilot 0.13.0 ships
46 built-in rules. Each finding carries category, severity, confidence,
recommendation, evidence, provenance, lifecycle, and signal-source metadata.

## Categories

| Category | Serialized value | Description |
|---|---|---|
| Architecture | `ARCHITECTURE` | Structural or design concerns |
| Code quality | `CODE_QUALITY` | Hygiene, maintainability, and runtime-risk issues |
| Testing | `TESTING` | Test coverage or quality gaps |
| Security | `SECURITY` | Potential vulnerabilities or exposed secrets |
| Framework | `FRAMEWORK` | Framework-specific configuration, migration, or runtime concerns |

## Severity And Confidence

| Severity | Serialized value |
|---|---|
| Info | `INFO` |
| Low | `LOW` |
| Medium | `MEDIUM` |
| High | `HIGH` |
| Critical | `CRITICAL` |

`severity` is the expected impact if the finding is true. `confidence` is how
certain RepoPilot is that the finding is a real problem in this context.

## Risk Prioritization

Severity is not the final sort key. RepoPilot also assigns each finding an
explainable `risk-v3` score and P0/P1/P2/P3 priority from severity, confidence,
Knowledge Engine rule calibration, file role, baseline status, review diff,
dependency graph impact, workspace hotspots, and repeated-pattern clusters.

See [Risk Engine](risk-engine.md) for the priority contract and signal families.

## Architecture Rules

| Rule ID | Default severity | What it detects |
|---|---:|---|
| `architecture.large-file` | Medium / High | Files above the configured non-empty LOC threshold. High is used for very large files. |
| `architecture.deep-nesting` | Low | Paths deeper than the configured directory depth threshold. |
| `architecture.deep-relative-imports` | Low / Medium | Imports crossing three or more parent directories, including JS/TS `../` imports and Rust `super::` chains. |
| `architecture.barrel-file-risk` | Low / Medium | Large `index.*` barrel files with many re-exports or wildcard exports. |
| `architecture.too-many-modules` | Medium | Directories containing more files than the configured module threshold. |
| `architecture.excessive-fan-out` | Medium | Files importing too many project-internal modules. |
| `architecture.high-instability-hub` | High | Files with high fan-in and high fan-out, making changes ripple broadly. |
| `architecture.circular-dependency` | High | Cycles in project-internal imports. |

Important defaults:

- `architecture.large-file`: 300 non-empty LOC; CLI override `--max-file-loc`.
- `architecture.too-many-modules`: 20 files per directory; CLI override `--max-directory-modules`.
- `architecture.deep-nesting`: 5 directory levels below scan root; CLI override `--max-directory-depth`.
- `architecture.excessive-fan-out`: `[architecture] max_fan_out`.
- `architecture.high-instability-hub`: `[architecture] instability_hub_min_fan_in` and `instability_hub_min_instability_pct`.

## Code Quality Rules

| Rule ID | Default severity | What it detects |
|---|---:|---|
| `code-quality.complex-file` | Medium / High | High branch-count density, computed as branch constructs x 1000 / LOC. |
| `code-quality.long-function` | Context-aware | Function bodies above the context-aware threshold. Production functions are usually medium/high-confidence; tests, React components/hooks, controllers, and lifecycle files can be lower-noise findings. |
| `code-marker.todo` | Low | `TODO` comments outside skipped docs/test paths. |
| `code-marker.fixme` | Medium | `FIXME` comments indicating known broken or problematic code. |
| `code-marker.hack` | Medium | `HACK` comments indicating workaround code. |

Important defaults:

- `code-quality.complex-file`: medium threshold 200 and high threshold 400 under `[code_quality]`.
- `code-quality.long-function`: `[architecture] max_function_lines`, default 50, with context-aware expansion for common framework/test roles.
- Long function support: Rust, Go, Python, TypeScript, JavaScript, Java, Kotlin, and C#.

## Language Runtime-Risk Rules

| Rule ID | Default severity | Signals |
|---|---:|---|
| `language.rust.panic-risk` | Context-aware | `rust.unwrap`, `rust.expect`, `rust.panic`, `rust.todo`, `rust.unimplemented` |
| `language.go.panic-exit-risk` | Context-aware | `go.panic`, `go.log-fatal`, `go.os-exit` |
| `language.python.exception-risk` | Context-aware | `python.broad-except`, `python.assert`, `python.not-implemented` |
| `language.javascript.runtime-exit-risk` | Context-aware | `js.process-exit`, `js.throw-error` |
| `language.managed.fatal-exception-risk` | Context-aware | `managed.fatal-exception`, `managed.not-implemented` |

These rules use file context from the Knowledge Engine. Test scaffolding, CLI
entrypoints, generated paths, and framework boundaries can reduce severity,
confidence, or suppress a finding. Domain, reusable library, and production code
usually keeps higher confidence.

## Security Rules

| Rule ID | Default severity | What it detects |
|---|---:|---|
| `security.secret-candidate` | High | High-entropy values or known secret-like patterns. Evidence masks secret values. |
| `security.private-key-candidate` | Critical | PEM private key headers. Key content is never included in evidence. |
| `security.env-file-committed` | Critical | Committed `.env`, `.env.local`, `.env.production`, `.env.staging`, or `.env.development` files. |

Security audits skip low-signal examples, fixtures, docs, lockfiles, and common
test paths where appropriate to reduce false positives.
For secret examples in application code or setup UI, prefer placeholders such as
`<OPENAI_API_KEY>`, `${OPENAI_API_KEY}`, `your-openai-api-key`,
`replace-with-*`, or `example-*`. Provider-looking or high-entropy values remain
high-severity findings and should be rotated if committed.

## Testing Rules

| Rule ID | Default severity | What it detects |
|---|---:|---|
| `testing.missing-test-folder` | Medium | No recognizable test directory such as `tests/`, `test/`, `__tests__/`, or `spec/`. |
| `testing.source-without-test` | Low | Source files without detectable test counterparts. |

`testing.source-without-test` skips wrapper and low-signal files such as
`mod.rs`, `lib.rs`, `main.rs`, generated files, type-only files, constants,
tokens, mocks, and framework setup files.

## JavaScript And React Framework Rules

| Rule ID | Default severity | What it detects |
|---|---:|---|
| `framework.js.var-declaration` | Low | `var` usage in JavaScript or TypeScript source. |
| `framework.js.console-log` | Low | `console.log` outside test files. |
| `framework.react.class-component` | Low | React class components. |
| `framework.react.prop-types` | Low | `prop-types` runtime checks in typed React projects. |

## React Native Rules

| Rule ID | Default severity | What it detects |
|---|---:|---|
| `framework.react-native.old-architecture` | Medium | No detected New Architecture enablement signal. |
| `framework.react-native.architecture-mismatch` | High | Android, iOS, or Expo New Architecture settings disagree. |
| `framework.react-native.hermes-disabled` | Low | Hermes explicitly disabled. |
| `framework.react-native.hermes-mismatch` | Medium | Hermes settings differ between Android and iOS. |
| `framework.react-native.codegen-missing` | Medium | Turbo Module or Fabric-like usage without `codegenConfig`. |
| `framework.react-native.async-storage-from-core` | High | `AsyncStorage` imported from `react-native` core. |
| `framework.react-native.old-react-navigation` | Medium | Legacy unscoped `react-navigation` imports. |
| `framework.react-native.direct-state-mutation` | High | Direct `this.state` mutation in class components. |
| `framework.react-native.inline-style` | Medium | JSX inline style objects such as `style={{ ... }}`. |
| `framework.react-native.deprecated-api` | High | Removed core APIs such as `ViewPagerAndroid`, `ToolbarAndroid`, `CheckBox`, or old platform widgets. |
| `framework.react-native.flatlist-missing-key` | Low | `FlatList` without `keyExtractor`. |

See [React Native Analysis](react-native.md) for supported project shapes, profile
fields, and limitations.

## React Native Dependency Health Rules

| Rule ID | Default severity | What it detects |
|---|---:|---|
| `framework.rn-async-storage-legacy` | Medium | Deprecated `@react-native-community/async-storage`. |
| `framework.rn-navigation-compat` | High | React Navigation version incompatible with the detected React Native version. |
| `framework.rn-reanimated-compat` | High | Reanimated version incompatible with the detected React Native version. |
| `framework.rn-gesture-handler-old` | High | Gesture Handler v1 with a React Native version that requires a newer line. |
| `framework.rn-new-arch-incompatible-dep` | Medium | Known dependency without React Native New Architecture support. |

## Django Rules

| Rule ID | Default severity | What it detects |
|---|---:|---|
| `framework.django.debug-true` | High | `DEBUG = True` in Django settings files. |
| `framework.django.missing-allowed-hosts` | High | `ALLOWED_HOSTS = []` in Django settings files. |
| `framework.django.raw-sql-query` | Medium | `cursor.execute(...)` calls that combine raw SQL with string formatting. |

## Finding Fields

Every finding includes these top-level fields:

| Field | Type | Description |
|---|---|---|
| `id` | string | Stable finding ID derived from rule, path, and line. |
| `rule_id` | string | Stable rule identifier. |
| `title` | string | Short human-readable summary. |
| `description` | string | Why the finding matters. |
| `recommendation` | string | Explicit remediation guidance. Older reports without this field still deserialize, but new reports include it. |
| `category` | string | One of `ARCHITECTURE`, `CODE_QUALITY`, `TESTING`, `SECURITY`, or `FRAMEWORK`. |
| `severity` | string | One of `INFO`, `LOW`, `MEDIUM`, `HIGH`, or `CRITICAL`. |
| `confidence` | string | One of `LOW`, `MEDIUM`, or `HIGH`; omitted values in old JSON reports are read as `MEDIUM`. |
| `docs_url` | string? | Link to documentation for the rule, when set. |
| `workspace_package` | string? | Package name in monorepos, when set. |
| `evidence` | array | One or more evidence locations. |

## Evidence

Each entry in the `evidence` array contains:

| Field | Type | Description |
|---|---|---|
| `path` | path | File that triggered the finding. |
| `line_start` | usize | First relevant line, 1-based. |
| `line_end` | usize? | Last relevant line, when available. |
| `snippet` | string | Source text or computed evidence at that location. |

## False-Positive Policy

Rules should be tuned with context before they are broadened. If a finding is a
false positive, prefer adding a narrow regression test, Knowledge Engine
override, or documented limitation over lowering the rule globally. RepoPilot
findings are review signals and should be used alongside language-native tools.
