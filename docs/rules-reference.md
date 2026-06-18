# RepoPilot Rules Reference

<!-- @generated from the rule registry — do not edit by hand. -->
<!-- Regenerate with `REPOPILOT_BLESS=1 cargo test --test rules_reference_doc`. -->

RepoPilot ships 52 rules across 5 categories. Every finding traces back to one of these rules. Severity and confidence below are the registry defaults: context (audit tiers, knowledge packs) may lower them, or raise them up to a rule's declared ceiling, but never past it.

## Architecture

### `architecture.barrel-file-risk` — Risky barrel file detected

- **Severity:** LOW
- **Confidence:** LOW
- **Lifecycle:** experimental
- **Signal source:** text-heuristic

An index file re-exports many modules or relies heavily on wildcard exports. Large barrel files can become unstable module hubs and make dependency boundaries harder to understand.

**Recommendation:** Split the barrel into smaller feature-level entrypoints or replace wildcard exports with explicit, stable public APIs.

### `architecture.circular-dependency` — Circular import dependency detected

- **Severity:** HIGH
- **Confidence:** HIGH
- **Lifecycle:** stable
- **Signal source:** import-graph

Two or more files import each other, forming a cycle. Circular dependencies make build order undefined, complicate testing, and prevent dead-code elimination.

**Recommendation:** Extract the shared logic into a third module that both files can import without creating a cycle.

**Known false positives:** Generated, fixture, and test-only import cycles should stay out of default scans; production source cycles are treated as actionable.

**Reference:** <https://github.com/MykytaStel/repopilot/blob/main/docs/rulesets.md#architecture>

### `architecture.dead-module` — Dead module detected

- **Severity:** LOW
- **Confidence:** HIGH
- **Lifecycle:** experimental
- **Signal source:** import-graph

This production file is not imported by any other project file and is not a known entrypoint. It may be dead code.

**Recommendation:** Remove the file if it is no longer used, or ensure it is exported in the package's public API.

**Known false positives:** Dynamic imports, dependency injection, and build-tool aliases create importers the graph cannot see. When the repository contains unresolved internal imports (relative paths, aliases, or workspace-package imports the resolver cannot wire up — common in monorepos) the finding is reported at Low confidence, and it is suppressed when an unresolved import matches the candidate file's name.

### `architecture.deep-directory-nesting` — Deep directory nesting detected

- **Severity:** LOW
- **Confidence:** MEDIUM
- **Lifecycle:** experimental
- **Signal source:** text-heuristic

A file is nested deeply within the directory structure. Deep directory nesting makes the codebase harder to navigate, import from, and maintain.

**Recommendation:** Simplify the directory structure or introduce path aliases to flatten the file layout. Adjust the max depth threshold using scan config.

### `architecture.deep-relative-imports` — Deep relative import found

- **Severity:** LOW
- **Confidence:** MEDIUM
- **Lifecycle:** preview
- **Signal source:** text-heuristic

A source file imports across three or more parent directories. Deep relative imports are fragile during refactors and often indicate missing module boundaries or aliases.

**Recommendation:** Introduce a stable module boundary, path alias, or facade module instead of importing through multiple parent directories.

### `architecture.excessive-fan-out` — File imports too many project-internal modules

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** stable
- **Signal source:** import-graph

This file depends on an unusually large number of other internal modules, making it a fragile integration point that breaks whenever any dependency changes.

**Recommendation:** Introduce an abstraction layer or facade to reduce the number of direct imports. Consider splitting responsibilities across multiple files.

**Known false positives:** Aggregator entrypoints can be acceptable when intentionally reviewed; default thresholds should be raised locally only after that decision.

**Reference:** <https://github.com/MykytaStel/repopilot/blob/main/docs/rulesets.md#architecture>

### `architecture.high-instability-hub` — High-instability hub: high fan-in and high fan-out

- **Severity:** HIGH
- **Confidence:** HIGH
- **Lifecycle:** stable
- **Signal source:** import-graph

This file is both widely imported (high fan-in) and depends on many other modules (high fan-out). Changes here ripple across the codebase with no stable upstream to absorb them.

**Recommendation:** Separate the stable, widely-imported interface from the volatile implementation details to reduce coupling.

**Known false positives:** Framework entrypoints and generated hubs can be expected; production modules with high fan-in and fan-out should be reviewed. When the repository contains unresolved relative imports, fan-in is a lower bound and the finding is reported at Medium confidence.

**Reference:** <https://github.com/MykytaStel/repopilot/blob/main/docs/rulesets.md#architecture>

### `architecture.large-file` — File exceeds recommended size

- **Severity:** MEDIUM
- **Confidence:** MEDIUM
- **Lifecycle:** experimental
- **Signal source:** text-heuristic

This file has more lines of code than the configured threshold. Large files accumulate responsibilities over time and make navigation and testing harder.

**Recommendation:** Split the file into smaller, focused modules. Use repopilot.toml `max_file_loc` to adjust the threshold.

### `architecture.layer-violation` — Layer violation detected

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** experimental
- **Signal source:** import-graph

A module imports another module from a higher layer than its own, against the order declared in `[[architecture.layers]]`. Opt-in: emits nothing unless layers are configured.

**Recommendation:** Invert the dependency using an interface, or move the shared logic into a layer at or below the importer.

### `architecture.package-boundary-violation` — Package boundary violation

- **Severity:** MEDIUM
- **Confidence:** MEDIUM
- **Lifecycle:** experimental
- **Signal source:** import-graph

A file imports a private module from another package instead of using its public API. Auto-enabled on a detected npm/pnpm/Cargo/Go workspace; can also be driven explicitly with `[architecture] package_roots`.

**Recommendation:** Import from the package's public barrel file (e.g. index.ts, mod.rs) instead of reaching into its internal implementation.

**Known false positives:** Public entry files (index.ts/js/tsx/jsx, mod.rs, lib.rs) are exempt, as are imports within the same package. Intentional deep imports in a private monorepo can be suppressed via feedback or by not configuring package boundaries. Workspace-detected boundaries report at High confidence; configured `package_roots` globs report at the Medium default.

### `architecture.test-leak` — Test code leaked into production

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** experimental
- **Signal source:** import-graph

A production module imports a test or fixture module.

**Recommendation:** Remove the test dependency from production code or extract the shared logic into a production utility.

### `architecture.too-many-modules` — Directory contains too many modules

- **Severity:** MEDIUM
- **Confidence:** MEDIUM
- **Lifecycle:** experimental
- **Signal source:** text-heuristic

A directory contains more files than the configured threshold, suggesting it may need to be broken into sub-packages.

**Recommendation:** Group related files into sub-directories. Adjust `max_directory_modules` in repopilot.toml if the current threshold is too strict.

## Code quality

### `code-marker.fixme` — FIXME marker found

- **Severity:** MEDIUM
- **Confidence:** LOW
- **Lifecycle:** experimental
- **Signal source:** text-heuristic

A FIXME comment marks known broken or problematic code that has not yet been addressed.

**Recommendation:** Fix the issue or file a bug report and reference its number in the comment.

### `code-marker.hack` — HACK marker found

- **Severity:** MEDIUM
- **Confidence:** LOW
- **Lifecycle:** experimental
- **Signal source:** text-heuristic

A HACK comment marks a workaround that bypasses a proper solution. Hacks tend to become permanent and break under refactoring.

**Recommendation:** Document why the hack exists and create a tracked issue to replace it with a proper implementation.

### `code-marker.todo` — TODO marker found

- **Severity:** LOW
- **Confidence:** LOW
- **Lifecycle:** experimental
- **Signal source:** text-heuristic

A TODO comment marks unfinished work. Unresolved TODOs accumulate as technical debt if not tracked in an issue tracker.

**Recommendation:** Convert the TODO into a tracked issue and reference the issue number in the comment, or resolve it immediately.

### `code-quality.complex-file` — File has high cyclomatic complexity

- **Severity:** MEDIUM
- **Confidence:** MEDIUM
- **Lifecycle:** preview
- **Signal source:** text-heuristic

The file's branch count density exceeds the complexity threshold, indicating too many execution paths. High complexity increases the defect rate and testing burden.

**Recommendation:** Extract conditionals into well-named helper functions. Prefer early returns to deeply nested if/else chains.

### `code-quality.complex-function` — Function has high cognitive complexity

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** ast

A function's control flow is deeply nested or branch-heavy, measured by a cognitive-complexity score that weights nesting depth rather than counting branches flatly. Deeply nested logic is disproportionately hard to read, test, and change.

**Recommendation:** Reduce nesting: extract nested blocks into well-named helper functions and prefer early returns/guard clauses over deep if/else and loop nesting.

**Known false positives:** A wide but flat dispatcher (a large switch/match with shallow arms) scores low by design, so it is not flagged. Nested closures/callbacks are scored independently rather than folded into their enclosing function. Cross-reference: `code-quality.complex-file` counts branches per file and does not weight nesting.

### `code-quality.deep-control-flow` — Deep control flow nesting detected

- **Severity:** LOW
- **Confidence:** MEDIUM
- **Lifecycle:** preview
- **Signal source:** ast

A source file contains deeply nested control flow blocks (if, loops, match, try). High nesting depth makes code hard to read, maintain, and test.

**Recommendation:** Extract nested blocks into separate helper functions or simplify the control flow.

### `code-quality.long-function` — Function body exceeds recommended length

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** ast

A function is longer than the configured line threshold. Long functions are harder to test, understand, and safely refactor.

**Recommendation:** Break the function into smaller, single-purpose helpers. Aim for functions that fit on a single screen.

**Known false positives:** Parseable languages (Rust, TypeScript/JavaScript, Python) measure real function spans from the syntax tree. Languages without a tree-sitter grammar (Go, Java, C#, Kotlin) fall back to a line/brace heuristic and are reported with text-heuristic provenance.

### `language.go.panic-exit-risk` — Risky Go panic or process exit usage

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** ast

Go panic and process-exit operations can terminate the program abruptly. Their risk depends on whether the file is test code, CLI boundary code, library code, or domain code.

**Recommendation:** Return errors from reusable code and reserve panic/log.Fatal/os.Exit for narrow process boundaries where callers cannot recover.

**Known false positives:** `panic`, `log.Fatal`/`log.Fatalf`, and `os.Exit` calls are matched from the parsed syntax tree, so the same text inside comments or string literals is not flagged. A line heuristic with text-heuristic provenance is used only when the file fails to parse.

### `language.javascript.runtime-exit-risk` — Risky JavaScript runtime exit or library throw

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** ast

Process exits and generic thrown errors have different risk at a CLI boundary than in reusable browser, Node, or package code.

**Recommendation:** Prefer returning typed errors, rejecting promises with actionable context, or centralising CLI exit handling at the entrypoint.

**Known false positives:** `process.exit` calls and `throw new Error(...)` are matched from the parsed syntax tree, so the same text inside comments or string literals is not flagged. A line heuristic with text-heuristic provenance is used only when the file fails to parse.

### `language.managed.fatal-exception-risk` — Risky JVM or .NET fatal exception placeholder

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** ast

Generic fatal exceptions and not-implemented placeholders in Java, Kotlin, or C# domain/library code can become runtime failures that callers cannot handle precisely.

**Recommendation:** Replace placeholders with implemented behaviour and use domain-specific exception or result types where callers need to recover.

**Known false positives:** Fatal exceptions and placeholder throw/TODO structures are matched from the parsed syntax tree, so the same text inside comments or string literals is not flagged. A line scanner with text-heuristic provenance is used only when the file fails to parse.

### `language.python.exception-risk` — Risky Python exception or assertion pattern

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** ast

Broad exception handlers, production asserts, and NotImplementedError placeholders can hide failures or ship incomplete behaviour. Their severity depends on test, script, and domain context.

**Recommendation:** Catch specific exceptions, use explicit runtime validation, and replace placeholders before production release.

**Known false positives:** Matches come from the parsed syntax tree (bare `except` clauses, `assert` statements, and `NotImplementedError`), so the same tokens inside comments or string literals are not flagged. A line heuristic with text-heuristic provenance is used only when the file fails to parse.

### `language.rust.panic-risk` — Risky Rust panic or unwrap usage

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** ast

Rust panic-style operations such as unwrap(), expect(), panic!, todo!, and unimplemented! can be risky in reusable production code. Their severity depends on whether the code is test code, CLI boundary code, library code, or domain code.

**Recommendation:** Use context-aware error handling. Prefer Result, ?, typed errors, validation, or explicit fallback behavior in production and library code.

**Known false positives:** `unwrap`/`expect`/`unwrap_err`/`expect_err` calls and `panic!`/`todo!`/`unimplemented!` macros are matched from the parsed syntax tree, so the same tokens inside comments or string literals (including multi-line raw strings) are not flagged, and infallible `write!`/`writeln!` result unwraps in report renderers are recognized structurally. Severity context (test/CLI/library/domain) and external-input escalation remain heuristic. A line scanner with text-heuristic provenance is used only when the file fails to parse.

## Testing

### `testing.missing-test-folder` — No test directory found in project

- **Severity:** MEDIUM
- **Confidence:** MEDIUM
- **Lifecycle:** experimental
- **Signal source:** text-heuristic

The project has no recognisable test directory (tests/, __tests__, spec/). Without tests, correctness can only be verified manually.

**Recommendation:** Create a test directory and add at least smoke tests for the core logic.

### `testing.source-without-test` — Source file has no corresponding test file

- **Severity:** LOW
- **Confidence:** LOW
- **Lifecycle:** experimental
- **Signal source:** text-heuristic

A source file has no matching test file. Untested code is more likely to regress during refactoring.

**Recommendation:** Add a test file alongside the source file, or co-locate tests within the same file following the project's testing conventions.

## Security

### `framework.django.debug-true` — DEBUG = True in Django settings

- **Severity:** HIGH
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** config-file

Django DEBUG mode exposes detailed error pages with stack traces, local variables, and settings values.

**Recommendation:** Set DEBUG = False for deployed environments and load debug mode only from local development configuration.

**Reference:** <https://docs.djangoproject.com/en/stable/ref/settings/#debug>

### `framework.django.missing-allowed-hosts` — ALLOWED_HOSTS is empty in Django settings

- **Severity:** HIGH
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** config-file

An empty ALLOWED_HOSTS setting leaves deployed Django services exposed to unsafe Host header handling.

**Recommendation:** Set ALLOWED_HOSTS to the explicit domain names and IP addresses the service should accept.

**Reference:** <https://docs.djangoproject.com/en/stable/ref/settings/#allowed-hosts>

### `framework.django.raw-sql-query` — Raw SQL with string formatting detected

- **Severity:** MEDIUM
- **Confidence:** MEDIUM
- **Lifecycle:** preview
- **Signal source:** text-heuristic

String formatting inside cursor.execute can turn user-controlled values into SQL injection risk.

**Recommendation:** Pass query parameters separately, for example cursor.execute(sql, [param]), so the database driver escapes values safely.

**Reference:** <https://docs.djangoproject.com/en/stable/topics/db/sql/#passing-parameters-into-raw>

### `security.env-file-committed` — Environment file committed to version control

- **Severity:** CRITICAL
- **Confidence:** HIGH
- **Lifecycle:** stable
- **Signal source:** config-file

A .env file containing environment variables has been committed. These files frequently contain secrets, API keys, or credentials that should never enter version control.

**Recommendation:** Add .env (and .env.*) to .gitignore, rotate any exposed credentials immediately, and use a secrets manager or CI environment variables instead.

**Known false positives:** Local example environment files should use sample names such as .env.example; committed .env variants are treated as sensitive by default.

**Reference:** <https://12factor.net/config>

### `security.private-key-candidate` — Possible private key in source file

- **Severity:** CRITICAL
- **Confidence:** HIGH
- **Lifecycle:** stable
- **Signal source:** text-heuristic

A PEM-encoded private key block was found in a source file. Committed private keys can be extracted from git history even after deletion.

**Recommendation:** Remove the key from the file immediately, rotate the key pair, and purge the git history using git-filter-repo or BFG Repo Cleaner.

**Known false positives:** Documentation examples and placeholder PEM blocks should live in docs or fixture paths and should not contain real key material.

**Reference:** <https://github.com/MykytaStel/repopilot/blob/main/docs/security.md#reporting-a-security-issue>

### `security.secret-candidate` — Possible hardcoded secret or API key

- **Severity:** HIGH
- **Confidence:** MEDIUM
- **Lifecycle:** preview
- **Signal source:** text-heuristic

A high-entropy string or a pattern matching a known secret format was found in source code. Hardcoded secrets are exposed to everyone with repository access.

**Recommendation:** Move the value to an environment variable or secrets manager. If this is a real credential that was committed, rotate it and consider the old value compromised. Use explicit placeholders such as `<OPENAI_API_KEY>` or `${OPENAI_API_KEY}` in examples.

**Known false positives:** Public labels, documented variable names, environment-variable references, and placeholders such as `your-openai-api-key`, `replace-with-*`, `example-*`, `<OPENAI_API_KEY>`, and `${OPENAI_API_KEY}` should not trigger; entropy and provider-looking token formats are both considered.

**Reference:** <https://github.com/MykytaStel/repopilot/blob/main/docs/security.md#secret-handling>

## Framework

### `framework.js.console-log` — console.log call left in source

- **Severity:** LOW
- **Confidence:** LOW
- **Lifecycle:** experimental
- **Signal source:** ast

A console.log statement was found outside of test files. Debug logging left in production code leaks information and adds noise.

**Recommendation:** Remove the console.log call or replace it with a structured logger that respects log levels.

### `framework.js.var-declaration` — var declaration in JavaScript/TypeScript file

- **Severity:** LOW
- **Confidence:** MEDIUM
- **Lifecycle:** preview
- **Signal source:** ast

`var` has function scope and is hoisted, which can produce subtle bugs. Modern JavaScript uses `const` and `let` instead.

**Recommendation:** Replace `var` with `const` for values that do not change, or `let` for variables that are reassigned.

**Reference:** <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/var>

### `framework.react-native.architecture-mismatch` — React Native New Architecture settings differ by platform

- **Severity:** HIGH
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** framework-detector

Android, iOS, or Expo configuration disagree about React Native New Architecture. Mismatched platforms produce inconsistent runtime behavior.

**Recommendation:** Align newArchEnabled across android/gradle.properties, ios/Podfile.properties.json, and app.json.

**Reference:** <https://reactnative.dev/docs/new-architecture-intro>

### `framework.react-native.async-storage-from-core` — AsyncStorage imported from 'react-native' core

- **Severity:** HIGH
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** ast

AsyncStorage was removed from react-native core in v0.60 and throws a runtime error on modern versions.

**Recommendation:** Replace with `import AsyncStorage from '@react-native-async-storage/async-storage'`.

**Reference:** <https://react-native-async-storage.github.io/async-storage/docs/install>

### `framework.react-native.codegen-missing` — React Native Codegen config is missing

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** framework-detector

The project uses Turbo Native Modules or Fabric components but package.json does not define codegenConfig.

**Recommendation:** Add codegenConfig to package.json so React Native can generate native interfaces consistently.

**Reference:** <https://reactnative.dev/docs/the-new-architecture/codegen>

### `framework.react-native.deprecated-api` — Deprecated React Native API

- **Severity:** HIGH
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** ast

A React Native API removed from core is in use. Replace with the community package equivalent.

**Recommendation:** Replace the deprecated API with its @react-native-community package equivalent.

**Reference:** <https://reactnative.dev/docs/out-of-tree-platforms>

### `framework.react-native.direct-state-mutation` — Direct mutation of this.state detected

- **Severity:** HIGH
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** ast

Directly assigning to this.state bypasses React change detection; the component will not re-render.

**Recommendation:** Use this.setState({ key: value }) in class components or the useState setter in function components.

**Reference:** <https://react.dev/reference/react/Component#setstate>

### `framework.react-native.flatlist-missing-key` — FlatList is missing keyExtractor

- **Severity:** LOW
- **Confidence:** MEDIUM
- **Lifecycle:** preview
- **Signal source:** ast

A FlatList without keyExtractor falls back to array index keys, breaking list reconciliation when items are reordered or removed.

**Recommendation:** Add keyExtractor={(item) => item.id.toString()} or an equivalent unique key.

**Reference:** <https://reactnative.dev/docs/flatlist#keyextractor>

### `framework.react-native.hermes-disabled` — Hermes JavaScript engine is disabled

- **Severity:** LOW
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** framework-detector

Hermes is explicitly disabled. Hermes reduces startup time by 2-3x and is the default engine since React Native 0.70.

**Recommendation:** Remove the hermes_enabled: false / enableHermes: false flag to enable Hermes.

**Reference:** <https://reactnative.dev/docs/hermes>

### `framework.react-native.hermes-mismatch` — Hermes settings differ between Android and iOS

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** framework-detector

Hermes is configured differently across platforms, causing platform-specific runtime and performance behavior.

**Recommendation:** Align Hermes settings so both Android and iOS use the same JS engine.

**Reference:** <https://reactnative.dev/docs/hermes>

### `framework.react-native.inline-style` — Inline style object in JSX

- **Severity:** MEDIUM
- **Confidence:** MEDIUM
- **Lifecycle:** preview
- **Signal source:** ast

Inline style objects create a new object on every render, defeating memoization in React.memo and PureComponent children.

**Recommendation:** Extract styles into a StyleSheet.create call outside the component.

**Reference:** <https://reactnative.dev/docs/stylesheet>

### `framework.react-native.old-architecture` — React Native New Architecture is not enabled

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** framework-detector

The project does not have newArchEnabled set. The New Architecture eliminates the async JS bridge and is required by an increasing number of libraries.

**Recommendation:** Set `"newArchEnabled": true` in app.json or react-native.config.js.

**Reference:** <https://reactnative.dev/docs/new-architecture-intro>

### `framework.react-native.old-react-navigation` — React Navigation v4 (unscoped package) detected

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** dependency-manifest

react-navigation (v4) is no longer maintained and is incompatible with React Native 0.70+.

**Recommendation:** Migrate to @react-navigation/native v5/v6 following the official migration guide.

**Reference:** <https://reactnavigation.org/docs/getting-started>

### `framework.react.class-component` — React class component in use

- **Severity:** LOW
- **Confidence:** MEDIUM
- **Lifecycle:** preview
- **Signal source:** ast

Class components are the legacy React API. Function components with hooks are now the recommended approach and are better supported by the React compiler.

**Recommendation:** Migrate the class component to a function component using hooks (useState, useEffect, etc.).

**Reference:** <https://react.dev/reference/react/Component>

### `framework.react.prop-types` — PropTypes runtime type checking in use

- **Severity:** LOW
- **Confidence:** MEDIUM
- **Lifecycle:** preview
- **Signal source:** ast

PropTypes adds runtime overhead and was removed from React 19. TypeScript or Flow provide better static type checking without runtime cost.

**Recommendation:** Replace PropTypes with TypeScript prop type annotations and remove the prop-types package.

**Reference:** <https://react.dev/blog/2024/04/25/react-19-upgrade-guide#removed-proptypes>

### `framework.rn-async-storage-legacy` — Deprecated @react-native-community/async-storage in use

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** dependency-manifest

`@react-native-community/async-storage` is unmaintained. The actively maintained fork is `@react-native-async-storage/async-storage`.

**Recommendation:** Run: `npm remove @react-native-community/async-storage && npm install @react-native-async-storage/async-storage`.

**Reference:** <https://react-native-async-storage.github.io/async-storage/docs/install>

### `framework.rn-gesture-handler-old` — react-native-gesture-handler v1 incompatible with React Native version

- **Severity:** HIGH
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** dependency-manifest

`react-native-gesture-handler` v1 does not support React Native ≥0.72. Gesture responder internals changed in 0.72.

**Recommendation:** Upgrade to `react-native-gesture-handler` v2+: `npm install react-native-gesture-handler@latest`.

**Reference:** <https://docs.swmansion.com/react-native-gesture-handler/docs/installation>

### `framework.rn-navigation-compat` — React Navigation version incompatible with React Native version

- **Severity:** HIGH
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** dependency-manifest

The installed version of `@react-navigation/native` is not compatible with the React Native version in use.

**Recommendation:** Upgrade to `@react-navigation/native` v6 or later: `npm install @react-navigation/native@latest`.

**Reference:** <https://reactnavigation.org/docs/getting-started>

### `framework.rn-new-arch-incompatible-dep` — Dependency does not support React Native New Architecture

- **Severity:** MEDIUM
- **Confidence:** MEDIUM
- **Lifecycle:** preview
- **Signal source:** dependency-manifest

A dependency in use has no New Architecture (TurboModules / Fabric) support and will break when the New Architecture is enabled.

**Recommendation:** Check the library's GitHub issues for a New Architecture migration path, or find an actively-maintained alternative.

**Reference:** <https://reactnative.dev/docs/new-architecture-intro>

### `framework.rn-reanimated-compat` — react-native-reanimated version incompatible with React Native version

- **Severity:** HIGH
- **Confidence:** HIGH
- **Lifecycle:** preview
- **Signal source:** dependency-manifest

`react-native-reanimated` v2 is not compatible with React Native ≥0.73. v3 introduced breaking changes to the worklet runtime.

**Recommendation:** Upgrade to `react-native-reanimated` v3+: `npm install react-native-reanimated@latest`.

**Reference:** <https://docs.swmansion.com/react-native-reanimated/docs/fundamentals/installation>
