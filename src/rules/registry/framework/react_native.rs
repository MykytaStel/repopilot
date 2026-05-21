use crate::findings::types::{Confidence, FindingCategory, Severity};
use crate::rules::metadata::RuleMetadata;
use crate::rules::{RuleLifecycle, SignalSource};

pub(crate) static REACT_NATIVE_RULES: &[RuleMetadata] = &[
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
];
