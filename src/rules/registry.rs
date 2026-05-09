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
        docs_url: None,
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
