# Language and Framework Support

RepoPilot combines generic repository heuristics with language and framework-aware
rules.

## Support tiers

### Tier 1 — rule-aware

These languages and ecosystems have deeper rule coverage:

- Rust
- JavaScript
- TypeScript
- React
- React Native
- Expo
- Python
- Go

### Tier 2 — context-aware / partial

These languages may have detection, import extraction, long-function checks, or
runtime-risk checks, but not full framework-specific coverage:

- Java
- Kotlin
- C#
- C
- C++

### Tier 3 — detect-only / generic

Other files can still contribute to repository size, scan scope, and generic
findings when supported by the scanner.

## Current rule families

### Architecture

- large files
- too many modules per directory
- deep nesting
- deep relative imports
- risky barrel files
- excessive fan-out
- high-instability hubs
- circular dependencies

### Code quality

- long functions
- complexity density
- TODO/FIXME/HACK markers
- language runtime-risk patterns

### Security

- hardcoded secret candidates
- private key candidates
- committed `.env` files

### Testing

- missing test folders
- source files without test counterparts

### Framework

- React Native architecture profile
- Hermes / New Architecture mismatch
- React Native deprecated APIs
- React Native dependency health
- React class components
- PropTypes in typed React projects
- JavaScript `var`
- debug `console.log`

## Framework detection

RepoPilot detects framework signals from project files such as:

- `package.json`
- React Native native configuration files
- Expo config
- `requirements.txt`
- `pyproject.toml`
- `go.mod`

Detected frameworks are used to reduce irrelevant findings. For example, React web
rules should not run on every React Native project just because React is installed.

## Rule philosophy

RepoPilot should not treat every paradigm as a smell.

Functional, object-oriented, procedural, and declarative code can all be valid.
Rules should use context and confidence instead of assuming one style is always
better.

Preferred rule design:

```text
evidence + context + severity + confidence + recommendation
```

## Limitations

RepoPilot is not a full compiler or type checker. Some rules are text-based or
heuristic. Findings should be treated as review signals, not absolute truth.

Use language-specific tools alongside RepoPilot:

- Rust: `cargo clippy`, `cargo test`
- TypeScript: `tsc`, ESLint
- Python: Ruff, Pyright, pytest
- Go: `go test`, `go vet`
- Java/Kotlin: Gradle/Maven checks
