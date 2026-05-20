use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;
use crate::rules::{RuleLifecycle, SignalSource};

pub(super) static RULES: &[RuleMetadata] = &[
    RuleMetadata {
        rule_id: "framework.react-native.inline-style",
        title: "Inline style object in JSX",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,
        docs_url: Some("https://reactnative.dev/docs/stylesheet"),
        description: "Inline style objects create a new object on every render, defeating memoization in React.memo and PureComponent children.",
        recommendation: Some("Extract styles into a StyleSheet.create call outside the component."),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react-native.deprecated-api",
        title: "Deprecated React Native API",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::Ast,
        docs_url: Some("https://reactnative.dev/docs/out-of-tree-platforms"),
        description: "A React Native API removed from core is in use. Replace with the community package equivalent.",
        recommendation: Some(
            "Replace the deprecated API with its @react-native-community package equivalent.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react-native.flatlist-missing-key",
        title: "FlatList is missing keyExtractor",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,
        docs_url: Some("https://reactnative.dev/docs/flatlist#keyextractor"),
        description: "A FlatList without keyExtractor falls back to array index keys, breaking list reconciliation when items are reordered or removed.",
        recommendation: Some(
            "Add keyExtractor={(item) => item.id.toString()} or an equivalent unique key.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react-native.async-storage-from-core",
        title: "AsyncStorage imported from 'react-native' core",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::Ast,
        docs_url: Some("https://react-native-async-storage.github.io/async-storage/docs/install"),
        description: "AsyncStorage was removed from react-native core in v0.60 and throws a runtime error on modern versions.",
        recommendation: Some(
            "Replace with `import AsyncStorage from '@react-native-async-storage/async-storage'`.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react-native.old-react-navigation",
        title: "React Navigation v4 (unscoped package) detected",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::DependencyManifest,
        docs_url: Some("https://reactnavigation.org/docs/getting-started"),
        description: "react-navigation (v4) is no longer maintained and is incompatible with React Native 0.70+.",
        recommendation: Some(
            "Migrate to @react-navigation/native v5/v6 following the official migration guide.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react-native.direct-state-mutation",
        title: "Direct mutation of this.state detected",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::Ast,
        docs_url: Some("https://react.dev/reference/react/Component#setstate"),
        description: "Directly assigning to this.state bypasses React change detection; the component will not re-render.",
        recommendation: Some(
            "Use this.setState({ key: value }) in class components or the useState setter in function components.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react-native.old-architecture",
        title: "React Native New Architecture is not enabled",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::FrameworkDetector,
        docs_url: Some("https://reactnative.dev/docs/new-architecture-intro"),
        description: "The project does not have newArchEnabled set. The New Architecture eliminates the async JS bridge and is required by an increasing number of libraries.",
        recommendation: Some(
            "Set `\"newArchEnabled\": true` in app.json or react-native.config.js.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react-native.architecture-mismatch",
        title: "React Native New Architecture settings differ by platform",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::FrameworkDetector,
        docs_url: Some("https://reactnative.dev/docs/new-architecture-intro"),
        description: "Android, iOS, or Expo configuration disagree about React Native New Architecture. Mismatched platforms produce inconsistent runtime behavior.",
        recommendation: Some(
            "Align newArchEnabled across android/gradle.properties, ios/Podfile.properties.json, and app.json.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react-native.hermes-mismatch",
        title: "Hermes settings differ between Android and iOS",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::FrameworkDetector,
        docs_url: Some("https://reactnative.dev/docs/hermes"),
        description: "Hermes is configured differently across platforms, causing platform-specific runtime and performance behavior.",
        recommendation: Some(
            "Align Hermes settings so both Android and iOS use the same JS engine.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react-native.hermes-disabled",
        title: "Hermes JavaScript engine is disabled",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::FrameworkDetector,
        docs_url: Some("https://reactnative.dev/docs/hermes"),
        description: "Hermes is explicitly disabled. Hermes reduces startup time by 2-3x and is the default engine since React Native 0.70.",
        recommendation: Some(
            "Remove the hermes_enabled: false / enableHermes: false flag to enable Hermes.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react-native.codegen-missing",
        title: "React Native Codegen config is missing",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::FrameworkDetector,
        docs_url: Some("https://reactnative.dev/docs/the-new-architecture/codegen"),
        description: "The project uses Turbo Native Modules or Fabric components but package.json does not define codegenConfig.",
        recommendation: Some(
            "Add codegenConfig to package.json so React Native can generate native interfaces consistently.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.js.var-declaration",
        title: "var declaration in JavaScript/TypeScript file",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,
        docs_url: Some(
            "https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/var",
        ),
        description: "`var` has function scope and is hoisted, which can produce subtle bugs. Modern JavaScript uses `const` and `let` instead.",
        recommendation: Some(
            "Replace `var` with `const` for values that do not change, or `let` for variables that are reassigned.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.js.console-log",
        title: "console.log call left in source",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        default_confidence: Confidence::Low,
        lifecycle: RuleLifecycle::Experimental,
        signal_source: SignalSource::Ast,
        docs_url: None,
        description: "A console.log statement was found outside of test files. Debug logging left in production code leaks information and adds noise.",
        recommendation: Some(
            "Remove the console.log call or replace it with a structured logger that respects log levels.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react.class-component",
        title: "React class component in use",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,
        docs_url: Some("https://react.dev/reference/react/Component"),
        description: "Class components are the legacy React API. Function components with hooks are now the recommended approach and are better supported by the React compiler.",
        recommendation: Some(
            "Migrate the class component to a function component using hooks (useState, useEffect, etc.).",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.react.prop-types",
        title: "PropTypes runtime type checking in use",
        category: FindingCategory::Framework,
        default_severity: Severity::Low,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::Ast,
        docs_url: Some(
            "https://react.dev/blog/2024/04/25/react-19-upgrade-guide#removed-proptypes",
        ),
        description: "PropTypes adds runtime overhead and was removed from React 19. TypeScript or Flow provide better static type checking without runtime cost.",
        recommendation: Some(
            "Replace PropTypes with TypeScript prop type annotations and remove the prop-types package.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.rn-async-storage-legacy",
        title: "Deprecated @react-native-community/async-storage in use",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::DependencyManifest,
        docs_url: Some("https://react-native-async-storage.github.io/async-storage/docs/install"),
        description: "`@react-native-community/async-storage` is unmaintained. The actively maintained fork is `@react-native-async-storage/async-storage`.",
        recommendation: Some(
            "Run: `npm remove @react-native-community/async-storage && npm install @react-native-async-storage/async-storage`.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.rn-navigation-compat",
        title: "React Navigation version incompatible with React Native version",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::DependencyManifest,
        docs_url: Some("https://reactnavigation.org/docs/getting-started"),
        description: "The installed version of `@react-navigation/native` is not compatible with the React Native version in use.",
        recommendation: Some(
            "Upgrade to `@react-navigation/native` v6 or later: `npm install @react-navigation/native@latest`.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.rn-reanimated-compat",
        title: "react-native-reanimated version incompatible with React Native version",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::DependencyManifest,
        docs_url: Some(
            "https://docs.swmansion.com/react-native-reanimated/docs/fundamentals/installation",
        ),
        description: "`react-native-reanimated` v2 is not compatible with React Native ≥0.73. v3 introduced breaking changes to the worklet runtime.",
        recommendation: Some(
            "Upgrade to `react-native-reanimated` v3+: `npm install react-native-reanimated@latest`.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.rn-gesture-handler-old",
        title: "react-native-gesture-handler v1 incompatible with React Native version",
        category: FindingCategory::Framework,
        default_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::DependencyManifest,
        docs_url: Some("https://docs.swmansion.com/react-native-gesture-handler/docs/installation"),
        description: "`react-native-gesture-handler` v1 does not support React Native ≥0.72. Gesture responder internals changed in 0.72.",
        recommendation: Some(
            "Upgrade to `react-native-gesture-handler` v2+: `npm install react-native-gesture-handler@latest`.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.rn-new-arch-incompatible-dep",
        title: "Dependency does not support React Native New Architecture",
        category: FindingCategory::Framework,
        default_severity: Severity::Medium,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::DependencyManifest,
        docs_url: Some("https://reactnative.dev/docs/new-architecture-intro"),
        description: "A dependency in use has no New Architecture (TurboModules / Fabric) support and will break when the New Architecture is enabled.",
        recommendation: Some(
            "Check the library's GitHub issues for a New Architecture migration path, or find an actively-maintained alternative.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.django.debug-true",
        title: "DEBUG = True in Django settings",
        category: FindingCategory::Security,
        default_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::ConfigFile,
        docs_url: Some("https://docs.djangoproject.com/en/stable/ref/settings/#debug"),
        description: "Django DEBUG mode exposes detailed error pages with stack traces, local variables, and settings values.",
        recommendation: Some(
            "Set DEBUG = False for deployed environments and load debug mode only from local development configuration.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.django.missing-allowed-hosts",
        title: "ALLOWED_HOSTS is empty in Django settings",
        category: FindingCategory::Security,
        default_severity: Severity::High,
        default_confidence: Confidence::High,
        lifecycle: RuleLifecycle::Stable,
        signal_source: SignalSource::ConfigFile,
        docs_url: Some("https://docs.djangoproject.com/en/stable/ref/settings/#allowed-hosts"),
        description: "An empty ALLOWED_HOSTS setting leaves deployed Django services exposed to unsafe Host header handling.",
        recommendation: Some(
            "Set ALLOWED_HOSTS to the explicit domain names and IP addresses the service should accept.",
        ),
        ..RuleMetadata::DEFAULT
    },
    RuleMetadata {
        rule_id: "framework.django.raw-sql-query",
        title: "Raw SQL with string formatting detected",
        category: FindingCategory::Security,
        default_severity: Severity::Medium,
        default_confidence: Confidence::Medium,
        lifecycle: RuleLifecycle::Preview,
        signal_source: SignalSource::TextHeuristic,
        docs_url: Some(
            "https://docs.djangoproject.com/en/stable/topics/db/sql/#passing-parameters-into-raw",
        ),
        description: "String formatting inside cursor.execute can turn user-controlled values into SQL injection risk.",
        recommendation: Some(
            "Pass query parameters separately, for example cursor.execute(sql, [param]), so the database driver escapes values safely.",
        ),
        ..RuleMetadata::DEFAULT
    },
];
