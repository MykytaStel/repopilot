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
| Framework | `FRAMEWORK` | Framework-specific configuration, migration, or runtime concerns |

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
| Severity | `CRITICAL` |
| Description | A `.env`, `.env.local`, `.env.production`, `.env.staging`, or `.env.development` file is present in the scanned tree. Environment files often contain secrets and must not be committed. |

### `code-quality.complex-file`

| Field | Value |
|---|---|
| Category | `CODE_QUALITY` |
| Severity | `MEDIUM` when density ≥ medium threshold; `HIGH` when density ≥ high threshold |
| Default thresholds | Medium: 200, High: 400 (branch constructs × 1000 / LOC) |
| Config keys | `[code_quality] complexity_medium_threshold`, `complexity_high_threshold` |
| Description | The file has a high branching density. Density is computed as `(branch_count × 1000) / lines_of_code` where branch constructs include `if`, `else`, `for`, `while`, `match`, `switch`, `case`, `catch`, `&&`, and `\|\|`. Values above the medium threshold suggest tangled logic; values above the high threshold indicate files likely needing to be split. |

Files with fewer than 10 non-empty LOC are skipped. Supported languages: Rust, Go, Python, TypeScript, JavaScript, Java, Kotlin, C, C++.

### `code-quality.long-function`

| Field | Value |
|---|---|
| Category | `CODE_QUALITY` |
| Severity | `MEDIUM` |
| Default threshold | 50 lines per function body |
| Config key | `[architecture] max_function_lines` |
| Description | A function body exceeds the configured line threshold. Long functions are harder to test, review, and reason about — consider extracting helpers or splitting logic into smaller units. |

Supported languages: Rust, Go, Python, TypeScript, JavaScript.

### `architecture.excessive-fan-out`

| Field | Value |
|---|---|
| Category | `ARCHITECTURE` |
| Severity | `MEDIUM` |
| Default threshold | 15 imported project files |
| Config key | `[architecture] max_fan_out` |
| Description | This file imports more project-internal files than the configured threshold. High fan-out increases the blast radius of changes and suggests the file may be accumulating unrelated responsibilities. |

Only project-internal imports are counted; stdlib and third-party imports are excluded.

### `architecture.high-instability-hub`

| Field | Value |
|---|---|
| Category | `ARCHITECTURE` |
| Severity | `HIGH` |
| Default thresholds | Minimum fan-in: 5; minimum instability: 75% |
| Config keys | `[architecture] instability_hub_min_fan_in`, `instability_hub_min_instability_pct` |
| Description | This file is imported by many files (high fan-in) but itself imports heavily from other files (high fan-out), resulting in high instability. An instability hub amplifies change propagation — modifications here risk breaking many dependents, yet it has little stability anchoring it. |

Instability is computed as `fan_out / (fan_in + fan_out)` expressed as a percentage.

### `architecture.circular-dependency`

| Field | Value |
|---|---|
| Category | `ARCHITECTURE` |
| Severity | `HIGH` |
| Description | A cycle of project-internal imports was detected. Circular dependencies prevent files from being compiled or tested in isolation, complicate dependency injection, and are a strong signal of tangled module boundaries. |

Cycles are deduplicated and reported once per unique set of files involved. Supported languages: Rust, TypeScript, JavaScript, Python, Go.

### `framework.react-native.old-architecture`

| Field | Value |
|---|---|
| Category | `FRAMEWORK` |
| Severity | `MEDIUM` |
| Description | React Native New Architecture is not enabled by any detected Android, iOS, or Expo configuration signal. |

### `framework.react-native.architecture-mismatch`

| Field | Value |
|---|---|
| Category | `FRAMEWORK` |
| Severity | `HIGH` |
| Description | Android, iOS, or Expo New Architecture settings disagree. Align the settings before release so all builds use the same runtime. |

### `framework.react-native.hermes-disabled`

| Field | Value |
|---|---|
| Category | `FRAMEWORK` |
| Severity | `LOW` |
| Description | Hermes is explicitly disabled in native React Native configuration. |

### `framework.react-native.hermes-mismatch`

| Field | Value |
|---|---|
| Category | `FRAMEWORK` |
| Severity | `MEDIUM` |
| Description | Hermes settings differ between Android and iOS. This can create platform-specific runtime and performance behavior. |

### `framework.react-native.codegen-missing`

| Field | Value |
|---|---|
| Category | `FRAMEWORK` |
| Severity | `MEDIUM` |
| Description | Turbo Module or Fabric-like usage was detected but `package.json` does not define `codegenConfig`. |

### `framework.react-native.async-storage-from-core`

| Field | Value |
|---|---|
| Category | `FRAMEWORK` |
| Severity | `HIGH` |
| Description | `AsyncStorage` is imported from `react-native` core instead of `@react-native-async-storage/async-storage`. |

### `framework.react-native.old-react-navigation`

| Field | Value |
|---|---|
| Category | `FRAMEWORK` |
| Severity | `MEDIUM` |
| Description | Legacy `react-navigation` v4 import was detected. Prefer the scoped `@react-navigation/*` packages. |

### `framework.react-native.direct-state-mutation`

| Field | Value |
|---|---|
| Category | `FRAMEWORK` |
| Severity | `HIGH` |
| Description | A class component mutates `this.state` directly instead of using `setState`. |

### `framework.react-native.inline-style`

| Field | Value |
|---|---|
| Category | `FRAMEWORK` |
| Severity | `MEDIUM` |
| Docs | https://reactnative.dev/docs/stylesheet |
| Description | A JSX element uses an inline style object (`style={{ ... }}`). Inline objects are reallocated on every render, defeating memoization. Extract styles with `StyleSheet.create`. |

### `framework.react-native.deprecated-api`

| Field | Value |
|---|---|
| Category | `FRAMEWORK` |
| Severity | `HIGH` |
| Docs | https://reactnative.dev/docs/out-of-tree-platforms |
| Description | A React Native core API that was removed in RN 0.65+ was detected (`ViewPagerAndroid`, `ToolbarAndroid`, `DatePickerAndroid`, `TimePickerAndroid`, `MaskedViewIOS`, `ProgressBarAndroid`, `ProgressViewIOS`, `SegmentedControlIOS`, or `CheckBox`). Replace with the corresponding community package. |

### `framework.react-native.flatlist-missing-key`

| Field | Value |
|---|---|
| Category | `FRAMEWORK` |
| Severity | `LOW` |
| Docs | https://reactnative.dev/docs/flatlist#keyextractor |
| Description | A `FlatList` component is missing the `keyExtractor` prop. Without it, React Native falls back to array index keys, breaking list reconciliation when items change order. |

See [React Native Analysis](react-native.md) for supported project shapes, profile fields, and limitations.

## Finding fields

Every finding includes these top-level fields:

| Field | Type | Description |
|---|---|---|
| `id` | string | Stable finding ID derived from rule + path + line |
| `rule_id` | string | Stable rule identifier (e.g. `security.private-key-candidate`) |
| `title` | string | Short human-readable summary |
| `description` | string | Full explanation with remediation steps |
| `category` | string | One of `ARCHITECTURE`, `CODE_QUALITY`, `TESTING`, `SECURITY`, `FRAMEWORK` |
| `severity` | string | One of `INFO`, `LOW`, `MEDIUM`, `HIGH`, `CRITICAL` |
| `docs_url` | string? | Link to official documentation for the rule (omitted when not set) |
| `workspace_package` | string? | Package name in monorepos (omitted for flat projects) |
| `evidence` | array | One or more evidence locations (see below) |

## Evidence

Each entry in the `evidence` array contains:

| Field | Type | Description |
|---|---|---|
| `path` | path | File that triggered the finding |
| `line_start` | usize | First relevant line (1-based) |
| `line_end` | usize? | Last relevant line (optional) |
| `snippet` | string | Source text at that location |
