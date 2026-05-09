use crate::findings::types::{FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;

static RULES: &[RuleMetadata] = &[
    RuleMetadata {
        rule_id: "framework.react-native.inline-style",
        title: "Inline style object in JSX",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        docs_url: Some("https://reactnative.dev/docs/stylesheet"),
        description: "Inline style objects create a new object on every render, defeating memoization in React.memo and PureComponent children.",
        recommendation: Some("Extract styles into a StyleSheet.create call outside the component."),
    },
    RuleMetadata {
        rule_id: "framework.react-native.deprecated-api",
        title: "Deprecated React Native API",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        docs_url: Some("https://reactnative.dev/docs/out-of-tree-platforms"),
        description: "A React Native API removed from core is in use. Replace with the community package equivalent.",
        recommendation: Some(
            "Replace the deprecated API with its @react-native-community package equivalent.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.react-native.flatlist-missing-key",
        title: "FlatList is missing keyExtractor",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        docs_url: Some("https://reactnative.dev/docs/flatlist#keyextractor"),
        description: "A FlatList without keyExtractor falls back to array index keys, breaking list reconciliation when items are reordered or removed.",
        recommendation: Some(
            "Add keyExtractor={(item) => item.id.toString()} or an equivalent unique key.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.react-native.async-storage-from-core",
        title: "AsyncStorage imported from 'react-native' core",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        docs_url: Some("https://react-native-async-storage.github.io/async-storage/docs/install"),
        description: "AsyncStorage was removed from react-native core in v0.60 and throws a runtime error on modern versions.",
        recommendation: Some(
            "Replace with `import AsyncStorage from '@react-native-async-storage/async-storage'`.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.react-native.old-react-navigation",
        title: "React Navigation v4 (unscoped package) detected",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        docs_url: Some("https://reactnavigation.org/docs/getting-started"),
        description: "react-navigation (v4) is no longer maintained and is incompatible with React Native 0.70+.",
        recommendation: Some(
            "Migrate to @react-navigation/native v5/v6 following the official migration guide.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.react-native.direct-state-mutation",
        title: "Direct mutation of this.state detected",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        docs_url: Some("https://react.dev/reference/react/Component#setstate"),
        description: "Directly assigning to this.state bypasses React change detection; the component will not re-render.",
        recommendation: Some(
            "Use this.setState({ key: value }) in class components or the useState setter in function components.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.react-native.old-architecture",
        title: "React Native New Architecture is not enabled",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        docs_url: Some("https://reactnative.dev/docs/new-architecture-intro"),
        description: "The project does not have newArchEnabled set. The New Architecture eliminates the async JS bridge and is required by an increasing number of libraries.",
        recommendation: Some(
            "Set `\"newArchEnabled\": true` in app.json or react-native.config.js.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.react-native.architecture-mismatch",
        title: "React Native New Architecture settings differ by platform",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        docs_url: Some("https://reactnative.dev/docs/new-architecture-intro"),
        description: "Android, iOS, or Expo configuration disagree about React Native New Architecture. Mismatched platforms produce inconsistent runtime behavior.",
        recommendation: Some(
            "Align newArchEnabled across android/gradle.properties, ios/Podfile.properties.json, and app.json.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.react-native.hermes-mismatch",
        title: "Hermes settings differ between Android and iOS",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        docs_url: Some("https://reactnative.dev/docs/hermes"),
        description: "Hermes is configured differently across platforms, causing platform-specific runtime and performance behavior.",
        recommendation: Some(
            "Align Hermes settings so both Android and iOS use the same JS engine.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.react-native.hermes-disabled",
        title: "Hermes JavaScript engine is disabled",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        docs_url: Some("https://reactnative.dev/docs/hermes"),
        description: "Hermes is explicitly disabled. Hermes reduces startup time by 2-3x and is the default engine since React Native 0.70.",
        recommendation: Some(
            "Remove the hermes_enabled: false / enableHermes: false flag to enable Hermes.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.react-native.codegen-missing",
        title: "React Native Codegen config is missing",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        docs_url: Some("https://reactnative.dev/docs/the-new-architecture/codegen"),
        description: "The project uses Turbo Native Modules or Fabric components but package.json does not define codegenConfig.",
        recommendation: Some(
            "Add codegenConfig to package.json so React Native can generate native interfaces consistently.",
        ),
    },
    // ── Architecture ──────────────────────────────────────────────────────────
    RuleMetadata {
        rule_id: "architecture.large-file",
        title: "File exceeds recommended size",
        category: FindingCategory::Architecture,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "This file has more lines of code than the configured threshold. Large files accumulate responsibilities over time and make navigation and testing harder.",
        recommendation: Some(
            "Split the file into smaller, focused modules. Use repopilot.toml `max_file_loc` to adjust the threshold.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.deep-nesting",
        title: "Directory nesting exceeds recommended depth",
        category: FindingCategory::Architecture,
        default_severity: Severity::Low,
        docs_url: None,
        description: "A directory path is deeper than the configured nesting threshold. Deeply nested structures make file discovery and import paths harder to maintain.",
        recommendation: Some(
            "Flatten the directory structure or consolidate related modules at a higher level.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.too-many-modules",
        title: "Directory contains too many modules",
        category: FindingCategory::Architecture,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "A directory contains more files than the configured threshold, suggesting it may need to be broken into sub-packages.",
        recommendation: Some(
            "Group related files into sub-directories. Adjust `max_directory_modules` in repopilot.toml if the current threshold is too strict.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.circular-dependency",
        title: "Circular import dependency detected",
        category: FindingCategory::Architecture,
        default_severity: Severity::High,
        docs_url: None,
        description: "Two or more files import each other, forming a cycle. Circular dependencies make build order undefined, complicate testing, and prevent dead-code elimination.",
        recommendation: Some(
            "Extract the shared logic into a third module that both files can import without creating a cycle.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.excessive-fan-out",
        title: "File imports too many project-internal modules",
        category: FindingCategory::Architecture,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "This file depends on an unusually large number of other internal modules, making it a fragile integration point that breaks whenever any dependency changes.",
        recommendation: Some(
            "Introduce an abstraction layer or facade to reduce the number of direct imports. Consider splitting responsibilities across multiple files.",
        ),
    },
    RuleMetadata {
        rule_id: "architecture.high-instability-hub",
        title: "High-instability hub: high fan-in and high fan-out",
        category: FindingCategory::Architecture,
        default_severity: Severity::High,
        docs_url: None,
        description: "This file is both widely imported (high fan-in) and depends on many other modules (high fan-out). Changes here ripple across the codebase with no stable upstream to absorb them.",
        recommendation: Some(
            "Separate the stable, widely-imported interface from the volatile implementation details to reduce coupling.",
        ),
    },
    // ── Code Quality ──────────────────────────────────────────────────────────
    RuleMetadata {
        rule_id: "code-quality.complex-file",
        title: "File has high cyclomatic complexity",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "The file's branch count density exceeds the complexity threshold, indicating too many execution paths. High complexity increases the defect rate and testing burden.",
        recommendation: Some(
            "Extract conditionals into well-named helper functions. Prefer early returns to deeply nested if/else chains.",
        ),
    },
    RuleMetadata {
        rule_id: "code-quality.long-function",
        title: "Function body exceeds recommended length",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "A function is longer than the configured line threshold. Long functions are harder to test, understand, and safely refactor.",
        recommendation: Some(
            "Break the function into smaller, single-purpose helpers. Aim for functions that fit on a single screen.",
        ),
    },
    // ── Code Markers ──────────────────────────────────────────────────────────
    RuleMetadata {
        rule_id: "code-marker.todo",
        title: "TODO marker found",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Low,
        docs_url: None,
        description: "A TODO comment marks unfinished work. Unresolved TODOs accumulate as technical debt if not tracked in an issue tracker.",
        recommendation: Some(
            "Convert the TODO into a tracked issue and reference the issue number in the comment, or resolve it immediately.",
        ),
    },
    RuleMetadata {
        rule_id: "code-marker.fixme",
        title: "FIXME marker found",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "A FIXME comment marks known broken or problematic code that has not yet been addressed.",
        recommendation: Some(
            "Fix the issue or file a bug report and reference its number in the comment.",
        ),
    },
    RuleMetadata {
        rule_id: "code-marker.hack",
        title: "HACK marker found",
        category: FindingCategory::CodeQuality,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "A HACK comment marks a workaround that bypasses a proper solution. Hacks tend to become permanent and break under refactoring.",
        recommendation: Some(
            "Document why the hack exists and create a tracked issue to replace it with a proper implementation.",
        ),
    },
    // ── Security ─────────────────────────────────────────────────────────────
    RuleMetadata {
        rule_id: "security.env-file-committed",
        title: "Environment file committed to version control",
        category: FindingCategory::Security,
        default_severity: Severity::Critical,
        docs_url: Some("https://12factor.net/config"),
        description: "A .env file containing environment variables has been committed. These files frequently contain secrets, API keys, or credentials that should never enter version control.",
        recommendation: Some(
            "Add .env (and .env.*) to .gitignore, rotate any exposed credentials immediately, and use a secrets manager or CI environment variables instead.",
        ),
    },
    RuleMetadata {
        rule_id: "security.private-key-candidate",
        title: "Possible private key in source file",
        category: FindingCategory::Security,
        default_severity: Severity::Critical,
        docs_url: None,
        description: "A PEM-encoded private key block was found in a source file. Committed private keys can be extracted from git history even after deletion.",
        recommendation: Some(
            "Remove the key from the file immediately, rotate the key pair, and purge the git history using git-filter-repo or BFG Repo Cleaner.",
        ),
    },
    RuleMetadata {
        rule_id: "security.secret-candidate",
        title: "Possible hardcoded secret or API key",
        category: FindingCategory::Security,
        default_severity: Severity::High,
        docs_url: None,
        description: "A high-entropy string or a pattern matching a known secret format was found in source code. Hardcoded secrets are exposed to everyone with repository access.",
        recommendation: Some(
            "Move the value to an environment variable or secrets manager. If already committed, rotate the credential and consider the old value compromised.",
        ),
    },
    // ── Testing ──────────────────────────────────────────────────────────────
    RuleMetadata {
        rule_id: "testing.missing-test-folder",
        title: "No test directory found in project",
        category: FindingCategory::Testing,
        default_severity: Severity::Medium,
        docs_url: None,
        description: "The project has no recognisable test directory (tests/, __tests__, spec/). Without tests, correctness can only be verified manually.",
        recommendation: Some(
            "Create a test directory and add at least smoke tests for the core logic.",
        ),
    },
    RuleMetadata {
        rule_id: "testing.source-without-test",
        title: "Source file has no corresponding test file",
        category: FindingCategory::Testing,
        default_severity: Severity::Low,
        docs_url: None,
        description: "A source file has no matching test file. Untested code is more likely to regress during refactoring.",
        recommendation: Some(
            "Add a test file alongside the source file, or co-locate tests within the same file following the project's testing conventions.",
        ),
    },
    // ── Framework: JavaScript / React ────────────────────────────────────────
    RuleMetadata {
        rule_id: "framework.js.var-declaration",
        title: "var declaration in JavaScript/TypeScript file",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        docs_url: Some(
            "https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/var",
        ),
        description: "`var` has function scope and is hoisted, which can produce subtle bugs. Modern JavaScript uses `const` and `let` instead.",
        recommendation: Some(
            "Replace `var` with `const` for values that do not change, or `let` for variables that are reassigned.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.js.console-log",
        title: "console.log call left in source",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        docs_url: None,
        description: "A console.log statement was found outside of test files. Debug logging left in production code leaks information and adds noise.",
        recommendation: Some(
            "Remove the console.log call or replace it with a structured logger that respects log levels.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.react.class-component",
        title: "React class component in use",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        docs_url: Some("https://react.dev/reference/react/Component"),
        description: "Class components are the legacy React API. Function components with hooks are now the recommended approach and are better supported by the React compiler.",
        recommendation: Some(
            "Migrate the class component to a function component using hooks (useState, useEffect, etc.).",
        ),
    },
    RuleMetadata {
        rule_id: "framework.react.prop-types",
        title: "PropTypes runtime type checking in use",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        docs_url: Some(
            "https://react.dev/blog/2024/04/25/react-19-upgrade-guide#removed-proptypes",
        ),
        description: "PropTypes adds runtime overhead and was removed from React 19. TypeScript or Flow provide better static type checking without runtime cost.",
        recommendation: Some(
            "Replace PropTypes with TypeScript prop type annotations and remove the prop-types package.",
        ),
    },
    // ── Framework: React Native dependency health ─────────────────────────────
    RuleMetadata {
        rule_id: "framework.rn-async-storage-legacy",
        title: "Deprecated @react-native-community/async-storage in use",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        docs_url: Some("https://react-native-async-storage.github.io/async-storage/docs/install"),
        description: "`@react-native-community/async-storage` is unmaintained. The actively maintained fork is `@react-native-async-storage/async-storage`.",
        recommendation: Some(
            "Run: `npm remove @react-native-community/async-storage && npm install @react-native-async-storage/async-storage`.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.rn-navigation-compat",
        title: "React Navigation version incompatible with React Native version",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        docs_url: Some("https://reactnavigation.org/docs/getting-started"),
        description: "The installed version of `@react-navigation/native` is not compatible with the React Native version in use.",
        recommendation: Some(
            "Upgrade to `@react-navigation/native` v6 or later: `npm install @react-navigation/native@latest`.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.rn-reanimated-compat",
        title: "react-native-reanimated version incompatible with React Native version",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        docs_url: Some(
            "https://docs.swmansion.com/react-native-reanimated/docs/fundamentals/installation",
        ),
        description: "`react-native-reanimated` v2 is not compatible with React Native ≥0.73. v3 introduced breaking changes to the worklet runtime.",
        recommendation: Some(
            "Upgrade to `react-native-reanimated` v3+: `npm install react-native-reanimated@latest`.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.rn-gesture-handler-old",
        title: "react-native-gesture-handler v1 incompatible with React Native version",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        docs_url: Some("https://docs.swmansion.com/react-native-gesture-handler/docs/installation"),
        description: "`react-native-gesture-handler` v1 does not support React Native ≥0.72. Gesture responder internals changed in 0.72.",
        recommendation: Some(
            "Upgrade to `react-native-gesture-handler` v2+: `npm install react-native-gesture-handler@latest`.",
        ),
    },
    RuleMetadata {
        rule_id: "framework.rn-new-arch-incompatible-dep",
        title: "Dependency does not support React Native New Architecture",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        docs_url: Some("https://reactnative.dev/docs/new-architecture-intro"),
        description: "A dependency in use has no New Architecture (TurboModules / Fabric) support and will break when the New Architecture is enabled.",
        recommendation: Some(
            "Check the library's GitHub issues for a New Architecture migration path, or find an actively-maintained alternative.",
        ),
    },
];

pub fn lookup_rule_metadata(rule_id: &str) -> Option<&'static RuleMetadata> {
    RULES.iter().find(|r| r.rule_id == rule_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::types::{FindingCategory, Severity};

    #[test]
    fn known_rn_rule_returns_metadata() {
        let meta = lookup_rule_metadata("framework.react-native.inline-style");
        assert!(meta.is_some(), "inline-style rule must be in the registry");
        let meta = meta.unwrap();
        assert_eq!(meta.rule_id, "framework.react-native.inline-style");
        assert_eq!(meta.category, FindingCategory::Framework);
        assert_eq!(meta.default_severity, Severity::Medium);
    }

    #[test]
    fn unknown_rule_returns_none() {
        assert!(lookup_rule_metadata("nonexistent.rule.id").is_none());
        assert!(lookup_rule_metadata("").is_none());
    }

    #[test]
    fn inline_style_docs_url_matches_official_stylesheet_docs() {
        let meta = lookup_rule_metadata("framework.react-native.inline-style").unwrap();
        assert_eq!(
            meta.docs_url,
            Some("https://reactnative.dev/docs/stylesheet")
        );
    }

    #[test]
    fn async_storage_docs_url_matches_official_install_docs() {
        let meta = lookup_rule_metadata("framework.react-native.async-storage-from-core").unwrap();
        assert_eq!(
            meta.docs_url,
            Some("https://react-native-async-storage.github.io/async-storage/docs/install")
        );
    }

    #[test]
    fn flatlist_missing_key_docs_url_matches_flatlist_keyextractor() {
        let meta = lookup_rule_metadata("framework.react-native.flatlist-missing-key").unwrap();
        assert_eq!(
            meta.docs_url,
            Some("https://reactnative.dev/docs/flatlist#keyextractor")
        );
    }

    #[test]
    fn old_react_navigation_docs_url_matches_getting_started() {
        let meta = lookup_rule_metadata("framework.react-native.old-react-navigation").unwrap();
        assert_eq!(
            meta.docs_url,
            Some("https://reactnavigation.org/docs/getting-started")
        );
    }

    #[test]
    fn deprecated_api_severity_is_high() {
        let meta = lookup_rule_metadata("framework.react-native.deprecated-api").unwrap();
        assert_eq!(meta.default_severity, Severity::High);
    }

    #[test]
    fn direct_state_mutation_severity_is_high() {
        let meta = lookup_rule_metadata("framework.react-native.direct-state-mutation").unwrap();
        assert_eq!(meta.default_severity, Severity::High);
    }

    #[test]
    fn all_registered_rules_have_non_empty_description_and_title() {
        for rule in RULES {
            assert!(
                !rule.title.is_empty(),
                "rule {} has empty title",
                rule.rule_id
            );
            assert!(
                !rule.description.is_empty(),
                "rule {} has empty description",
                rule.rule_id
            );
        }
    }

    #[test]
    fn all_registered_rule_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for rule in RULES {
            assert!(
                seen.insert(rule.rule_id),
                "duplicate rule_id: {}",
                rule.rule_id
            );
        }
    }
}
