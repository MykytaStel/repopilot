use repopilot::findings::types::{Evidence, Finding, FindingCategory, Severity};
use repopilot::output::sarif::findings_to_sarif;
use repopilot::rules::{RuleMetadata, lookup_rule_metadata};
use std::path::PathBuf;

// ── lookup correctness ────────────────────────────────────────────────────────

#[test]
fn known_rn_rule_returns_some() {
    let meta = lookup_rule_metadata("framework.react-native.inline-style");
    assert!(
        meta.is_some(),
        "inline-style must be present in the registry"
    );
}

#[test]
fn unknown_rule_returns_none() {
    assert!(lookup_rule_metadata("does.not.exist").is_none());
}

#[test]
fn empty_rule_id_returns_none() {
    assert!(lookup_rule_metadata("").is_none());
}

// ── metadata field correctness ────────────────────────────────────────────────

#[test]
fn inline_style_metadata_fields() {
    let meta: &RuleMetadata = lookup_rule_metadata("framework.react-native.inline-style").unwrap();
    assert_eq!(meta.category, FindingCategory::Framework);
    assert_eq!(meta.default_severity, Severity::Medium);
    assert_eq!(
        meta.docs_url,
        Some("https://reactnative.dev/docs/stylesheet")
    );
    assert!(meta.recommendation.is_some());
}

#[test]
fn async_storage_docs_url_matches_official_docs() {
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

// ── SARIF integration ─────────────────────────────────────────────────────────

fn make_finding(rule_id: &str) -> Finding {
    Finding {
        id: String::new(),
        rule_id: rule_id.to_owned(),
        title: "Test finding".to_owned(),
        description: "A test description.".to_owned(),
        category: FindingCategory::Framework,
        severity: Severity::Medium,
        evidence: vec![Evidence {
            path: PathBuf::from("src/App.tsx"),
            line_start: 5,
            line_end: None,
            snippet: "style={{ color: 'red' }}".to_owned(),
        }],
        workspace_package: None,
        docs_url: None,
    }
}

#[test]
fn sarif_rule_includes_help_uri_from_registry() {
    let finding = make_finding("framework.react-native.inline-style");
    let root = PathBuf::from(".");
    let sarif = findings_to_sarif(&[finding], &root);
    let value = serde_json::to_value(&sarif).unwrap();

    let help_uri = &value["runs"][0]["tool"]["driver"]["rules"][0]["helpUri"];
    assert_eq!(
        help_uri.as_str(),
        Some("https://reactnative.dev/docs/stylesheet"),
        "helpUri must come from the registry for a known rule"
    );
}

#[test]
fn sarif_rule_includes_help_uri_from_finding_when_not_in_registry() {
    let mut finding = make_finding("custom.rule.not.in.registry");
    finding.docs_url = Some("https://example.com/my-rule".to_owned());
    let root = PathBuf::from(".");
    let sarif = findings_to_sarif(&[finding], &root);
    let value = serde_json::to_value(&sarif).unwrap();

    let help_uri = &value["runs"][0]["tool"]["driver"]["rules"][0]["helpUri"];
    assert_eq!(
        help_uri.as_str(),
        Some("https://example.com/my-rule"),
        "helpUri should fall back to finding.docs_url when the rule is not in the registry"
    );
}

#[test]
fn sarif_finding_without_metadata_serializes_safely() {
    let finding = make_finding("totally.unknown.rule");
    let root = PathBuf::from(".");
    let sarif = findings_to_sarif(&[finding], &root);
    let value = serde_json::to_value(&sarif).expect("serialization must not panic");

    // Rule entry present but helpUri must be absent (null or missing)
    let rules = &value["runs"][0]["tool"]["driver"]["rules"];
    assert!(rules.is_array() && !rules.as_array().unwrap().is_empty());
    let rule = &rules[0];
    assert!(
        rule["helpUri"].is_null(),
        "helpUri must be absent when neither registry nor finding.docs_url has a value"
    );
}

#[test]
fn sarif_registry_rule_overrides_finding_docs_url() {
    // Even if the finding carries its own docs_url, the registry's authoritative
    // URL should take precedence.
    let mut finding = make_finding("framework.react-native.inline-style");
    finding.docs_url = Some("https://example.com/wrong-url".to_owned());
    let root = PathBuf::from(".");
    let sarif = findings_to_sarif(&[finding], &root);
    let value = serde_json::to_value(&sarif).unwrap();

    let help_uri = &value["runs"][0]["tool"]["driver"]["rules"][0]["helpUri"];
    assert_eq!(
        help_uri.as_str(),
        Some("https://reactnative.dev/docs/stylesheet"),
        "registry docs_url must take precedence over finding.docs_url"
    );
}
