# RepoPilot React Native Sample Report

## Summary

- **Path:** `apps/mobile`
- **Files analyzed:** 184
- **Directories analyzed:** 38
- **Lines of code:** 18,420

## Frameworks

React Native 0.76.0 · Expo 53.0.0 · React 19.0.0

### React Native

- **Version:** 0.76.0
- **Project kind:** `ExpoPrebuild`
- **Package manager:** pnpm
- **iOS:** detected
- **Android:** detected
- **Android New Architecture:** enabled
- **iOS New Architecture:** disabled
- **Expo New Architecture:** enabled
- **Hermes:** unknown
- **Codegen config:** missing

## Findings

| Severity | Rule | Title | Evidence |
| --- | --- | --- | --- |
| HIGH | `framework.react-native.architecture-mismatch` | React Native New Architecture settings differ by platform | `ios/Podfile.properties.json:1` - android=enabled; ios=disabled; expo=enabled |
| MEDIUM | `framework.react-native.codegen-missing` | React Native Codegen config is missing | `package.json:1` - codegenConfig missing while Codegen usage was detected |
| HIGH | `framework.react-native.async-storage-from-core` | AsyncStorage imported from 'react-native' core | `src/storage.ts:1` - import { AsyncStorage } from 'react-native'; |

## CI Gate

Use `--baseline .repopilot/baseline.json --fail-on new-high` to fail only on new high or critical findings while accepting existing debt.
