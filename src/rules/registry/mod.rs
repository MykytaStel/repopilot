mod architecture;
mod code_quality;
mod framework;
mod language;
mod security;
mod testing;

use crate::rules::metadata::RuleMetadata;

const RULE_GROUPS: &[&[RuleMetadata]] = &[
    framework::RULES,
    architecture::RULES,
    code_quality::RULES,
    language::RULES,
    security::RULES,
    testing::RULES,
];

pub fn lookup_rule_metadata(rule_id: &str) -> Option<&'static RuleMetadata> {
    all_rules().find(|rule| rule.rule_id == rule_id)
}

fn all_rules() -> impl Iterator<Item = &'static RuleMetadata> {
    RULE_GROUPS.iter().flat_map(|rules| rules.iter())
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
        for rule in all_rules() {
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
        for rule in all_rules() {
            assert!(
                seen.insert(rule.rule_id),
                "duplicate rule_id: {}",
                rule.rule_id
            );
        }
    }
}
