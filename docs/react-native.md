# React Native Analysis

RepoPilot detects React Native and Expo projects from `package.json` and reports architecture signals that are useful during review and release preparation.

## What RepoPilot Detects

- React Native, Expo, React, and other JavaScript frameworks from dependencies.
- Bare React Native, Expo managed, and Expo prebuild project shapes.
- Android and iOS platform folders.
- Metro, `react-native.config.*`, Expo app config, and `codegenConfig`.
- New Architecture settings from `android/gradle.properties`, `ios/Podfile`, `ios/Podfile.properties.json`, and Expo config.
- Hermes settings from Android and iOS native config.
- Package manager lockfile signal: npm, pnpm, Yarn, or Bun.
- Workspace packages declared through npm/Yarn-style `workspaces`.

## React Native Findings

| Rule | Severity | Meaning |
| --- | --- | --- |
| `framework.react-native.old-architecture` | Medium | New Architecture is not enabled or no platform signal enables it. |
| `framework.react-native.architecture-mismatch` | High | Android, iOS, or Expo New Architecture settings disagree. |
| `framework.react-native.hermes-disabled` | Low | Hermes is explicitly disabled. |
| `framework.react-native.hermes-mismatch` | Medium | Android and iOS Hermes settings disagree. |
| `framework.react-native.codegen-missing` | Medium | Turbo Module or Fabric-like usage was detected but `codegenConfig` is missing. |
| `framework.react-native.async-storage-from-core` | High | `AsyncStorage` is imported from `react-native` core. |
| `framework.react-native.old-react-navigation` | Medium | Legacy `react-navigation` v4 import was detected. |
| `framework.react-native.direct-state-mutation` | High | Class component code mutates `this.state` directly. |
| `framework.react-native.inline-style` | Medium | Inline `style={{ ... }}` objects found in JSX — creates a new object on every render, defeating memoization. |
| `framework.react-native.deprecated-api` | High | Removed core API detected (`ViewPagerAndroid`, `ToolbarAndroid`, `DatePickerAndroid`, etc.). |
| `framework.react-native.flatlist-missing-key` | Low | `FlatList` without `keyExtractor` falls back to index-based keys, breaking reconciliation on reorder. |

## Recommended Commands

```bash
repopilot scan . --format markdown --output repopilot-report.md
repopilot review . --base origin/main --format markdown --output repopilot-review.md
repopilot scan . --baseline .repopilot/baseline.json --fail-on new-high
repopilot scan . --format sarif --output repopilot.sarif
```

## Report Example

Use RepoPilot as a local architecture health pass before a React Native release:

```bash
repopilot scan . --format markdown --output rn-architecture-health.md
repopilot ai context . --focus framework --budget 4k --output rn-ai-context.md
```

The report highlights Hermes/New Architecture mismatches, deprecated core APIs,
direct state mutation, missing Codegen config, dependency signals, and normal
repository risks such as large files or hardcoded secret candidates. Commit a
baseline for accepted migration debt, then gate pull requests on new high-risk
findings:

```bash
repopilot baseline create .
repopilot review . --base origin/main --baseline .repopilot/baseline.json --fail-on new-high
```

## Known Limitations

- JavaScript and TypeScript config files are detected with conservative heuristics, not executed.
- Workspace detection supports direct paths and one-level `packages/*` style patterns.
- Codegen detection looks for common Turbo Module and Fabric usage markers; it is not a full TypeScript parser.
- RepoPilot does not upload source code. Reports are generated locally.
