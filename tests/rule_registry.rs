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
        recommendation: Finding::recommendation_for_rule_id(rule_id),
        title: "Test finding".to_owned(),
        description: "A test description.".to_owned(),
        category: FindingCategory::Framework,
        severity: Severity::Medium,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: PathBuf::from("src/App.tsx"),
            line_start: 5,
            line_end: None,
            snippet: "style={{ color: 'red' }}".to_owned(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
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

// ── Registry coverage: all emitted rule IDs must be registered ────────────────

#[test]
fn all_architecture_rule_ids_are_registered() {
    for rule_id in &[
        "architecture.large-file",
        "architecture.deep-nesting",
        "architecture.deep-relative-imports",
        "architecture.barrel-file-risk",
        "architecture.too-many-modules",
        "architecture.circular-dependency",
        "architecture.excessive-fan-out",
        "architecture.high-instability-hub",
    ] {
        assert!(
            lookup_rule_metadata(rule_id).is_some(),
            "architecture rule not in registry: {rule_id}"
        );
    }
}

#[test]
fn all_code_quality_rule_ids_are_registered() {
    for rule_id in &[
        "code-quality.complex-file",
        "code-quality.long-function",
        "code-marker.todo",
        "code-marker.fixme",
        "code-marker.hack",
    ] {
        assert!(
            lookup_rule_metadata(rule_id).is_some(),
            "code-quality rule not in registry: {rule_id}"
        );
    }
}

#[test]
fn all_language_runtime_risk_rule_ids_are_registered() {
    for rule_id in &[
        "language.rust.panic-risk",
        "language.go.panic-exit-risk",
        "language.python.exception-risk",
        "language.javascript.runtime-exit-risk",
        "language.managed.fatal-exception-risk",
    ] {
        assert!(
            lookup_rule_metadata(rule_id).is_some(),
            "language runtime-risk rule not in registry: {rule_id}"
        );
    }
}

#[test]
fn all_security_rule_ids_are_registered() {
    for rule_id in &[
        "security.env-file-committed",
        "security.private-key-candidate",
        "security.secret-candidate",
    ] {
        assert!(
            lookup_rule_metadata(rule_id).is_some(),
            "security rule not in registry: {rule_id}"
        );
    }
}

#[test]
fn all_testing_rule_ids_are_registered() {
    for rule_id in &["testing.missing-test-folder", "testing.source-without-test"] {
        assert!(
            lookup_rule_metadata(rule_id).is_some(),
            "testing rule not in registry: {rule_id}"
        );
    }
}

#[test]
fn all_framework_js_react_rule_ids_are_registered() {
    for rule_id in &[
        "framework.js.var-declaration",
        "framework.js.console-log",
        "framework.react.class-component",
        "framework.react.prop-types",
    ] {
        assert!(
            lookup_rule_metadata(rule_id).is_some(),
            "framework JS/React rule not in registry: {rule_id}"
        );
    }
}

#[test]
fn all_framework_rn_dep_health_rule_ids_are_registered() {
    for rule_id in &[
        "framework.rn-async-storage-legacy",
        "framework.rn-navigation-compat",
        "framework.rn-reanimated-compat",
        "framework.rn-gesture-handler-old",
        "framework.rn-new-arch-incompatible-dep",
    ] {
        assert!(
            lookup_rule_metadata(rule_id).is_some(),
            "RN dep-health rule not in registry: {rule_id}"
        );
    }
}

#[test]
fn all_framework_django_rule_ids_are_registered() {
    for rule_id in &[
        "framework.django.debug-true",
        "framework.django.missing-allowed-hosts",
        "framework.django.raw-sql-query",
    ] {
        assert!(
            lookup_rule_metadata(rule_id).is_some(),
            "Django rule not in registry: {rule_id}"
        );
    }
}

#[test]
fn static_rule_metadata_severities_match_emitted_findings() {
    let expected = [
        ("architecture.large-file", Severity::Medium),
        ("architecture.deep-nesting", Severity::Low),
        ("architecture.deep-relative-imports", Severity::Low),
        ("architecture.barrel-file-risk", Severity::Low),
        ("architecture.too-many-modules", Severity::Medium),
        ("architecture.circular-dependency", Severity::High),
        ("architecture.excessive-fan-out", Severity::Medium),
        ("architecture.high-instability-hub", Severity::High),
        ("code-quality.complex-file", Severity::Medium),
        ("code-quality.long-function", Severity::Medium),
        ("language.rust.panic-risk", Severity::Medium),
        ("language.go.panic-exit-risk", Severity::Medium),
        ("language.python.exception-risk", Severity::Medium),
        ("language.javascript.runtime-exit-risk", Severity::Medium),
        ("language.managed.fatal-exception-risk", Severity::Medium),
        ("code-marker.todo", Severity::Low),
        ("code-marker.fixme", Severity::Medium),
        ("code-marker.hack", Severity::Medium),
        ("security.env-file-committed", Severity::Critical),
        ("security.private-key-candidate", Severity::Critical),
        ("security.secret-candidate", Severity::High),
        ("testing.missing-test-folder", Severity::Medium),
        ("testing.source-without-test", Severity::Low),
        ("framework.js.var-declaration", Severity::Low),
        ("framework.js.console-log", Severity::Low),
        ("framework.react.class-component", Severity::Low),
        ("framework.react.prop-types", Severity::Low),
        ("framework.react-native.inline-style", Severity::Medium),
        ("framework.react-native.deprecated-api", Severity::High),
        ("framework.react-native.flatlist-missing-key", Severity::Low),
        (
            "framework.react-native.async-storage-from-core",
            Severity::High,
        ),
        (
            "framework.react-native.old-react-navigation",
            Severity::Medium,
        ),
        (
            "framework.react-native.direct-state-mutation",
            Severity::High,
        ),
        ("framework.react-native.old-architecture", Severity::Medium),
        (
            "framework.react-native.architecture-mismatch",
            Severity::High,
        ),
        ("framework.react-native.hermes-mismatch", Severity::Medium),
        ("framework.react-native.hermes-disabled", Severity::Low),
        ("framework.react-native.codegen-missing", Severity::Medium),
        ("framework.rn-async-storage-legacy", Severity::Medium),
        ("framework.rn-navigation-compat", Severity::High),
        ("framework.rn-reanimated-compat", Severity::High),
        ("framework.rn-gesture-handler-old", Severity::High),
        ("framework.rn-new-arch-incompatible-dep", Severity::Medium),
        ("framework.django.debug-true", Severity::High),
        ("framework.django.missing-allowed-hosts", Severity::High),
        ("framework.django.raw-sql-query", Severity::Medium),
    ];

    for (rule_id, severity) in expected {
        let meta = lookup_rule_metadata(rule_id)
            .unwrap_or_else(|| panic!("rule not registered: {rule_id}"));
        assert_eq!(
            meta.default_severity, severity,
            "registry severity mismatch for {rule_id}"
        );
    }
}

// ── RN stability: metadata completeness ──────────────────────────────────────

#[test]
fn architecture_mismatch_has_docs_url() {
    let meta = lookup_rule_metadata("framework.react-native.architecture-mismatch").unwrap();
    assert!(
        meta.docs_url.is_some(),
        "architecture-mismatch must have a docs_url"
    );
}

#[test]
fn all_rn_rules_have_recommendation() {
    let rn_rule_ids = [
        "framework.react-native.inline-style",
        "framework.react-native.deprecated-api",
        "framework.react-native.flatlist-missing-key",
        "framework.react-native.async-storage-from-core",
        "framework.react-native.old-react-navigation",
        "framework.react-native.direct-state-mutation",
        "framework.react-native.old-architecture",
        "framework.react-native.architecture-mismatch",
        "framework.react-native.hermes-mismatch",
        "framework.react-native.hermes-disabled",
        "framework.react-native.codegen-missing",
        "framework.rn-async-storage-legacy",
        "framework.rn-navigation-compat",
        "framework.rn-reanimated-compat",
        "framework.rn-gesture-handler-old",
        "framework.rn-new-arch-incompatible-dep",
    ];
    for rule_id in &rn_rule_ids {
        let meta = lookup_rule_metadata(rule_id)
            .unwrap_or_else(|| panic!("RN rule not in registry: {rule_id}"));
        assert!(
            meta.recommendation.is_some(),
            "RN rule {rule_id} must have a recommendation"
        );
    }
}
