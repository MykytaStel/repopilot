use repopilot::config::model::{CriticalPathRule, RepoPilotConfig};
use repopilot::review::intent::{
    IntentStatus, evaluate_intent, load_intent_file, parse_intent_json, parse_intent_toml,
    validate_critical_paths, validate_intent,
};
use repopilot::review::proof::{
    ChangeProofContractDelta, ContractChangeKind, ContractConfidence, ContractFamily,
};
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn critical_paths() -> Vec<CriticalPathRule> {
    vec![CriticalPathRule {
        name: "authentication".to_string(),
        paths: vec!["src/auth/**".to_string()],
    }]
}

#[test]
fn intent_contract_parses_and_canonicalizes_bounded_lists() {
    let intent = parse_intent_toml(
        r#"
version = 1
summary = "Update auth"
paths = ["tests/auth/**", "src/auth/**", "src/auth/**"]
contract_families = ["public-symbol", "security-boundary", "public-symbol"]
critical_paths = ["authentication", "authentication"]
verification = ["unit", "unit"]
"#,
    )
    .expect("intent parses");

    assert_eq!(intent.paths, vec!["src/auth/**", "tests/auth/**"]);
    assert_eq!(
        intent.contract_families,
        vec![
            ContractFamily::PublicSymbol,
            ContractFamily::SecurityBoundary
        ]
    );
    assert_eq!(intent.critical_paths, vec!["authentication"]);
    assert_eq!(intent.verification, vec!["unit"]);
}

#[test]
fn supplied_intent_reports_unexpected_paths_families_critical_areas_and_checks() {
    let intent = parse_intent_toml(
        r#"
version = 1
paths = ["src/auth/**"]
contract_families = ["security-boundary"]
critical_paths = []
verification = ["unit"]
"#,
    )
    .expect("intent parses");
    let actual_families = vec![ContractFamily::PublicSymbol];
    let result = evaluate_intent(
        Some(&intent),
        &["src/auth/login.rs".into(), "src/billing/invoice.rs".into()],
        &actual_families,
        &critical_paths(),
        &[],
    );

    assert_eq!(result.status, IntentStatus::Drifted);
    assert_eq!(result.unexpected_paths, vec!["src/billing/invoice.rs"]);
    assert_eq!(result.unexpected_contract_families, actual_families);
    assert_eq!(result.unexpected_critical_paths, vec!["authentication"]);
    assert_eq!(result.missing_verification, vec!["unit"]);
}

#[test]
fn missing_intent_is_not_a_penalty_but_still_reports_critical_intersections() {
    let result = evaluate_intent(
        None,
        &["src/auth/login.rs".into()],
        &[],
        &critical_paths(),
        &[],
    );

    assert_eq!(result.status, IntentStatus::NotSupplied);
    assert!(result.unexpected_paths.is_empty());
    assert_eq!(result.critical_path_matches[0].name, "authentication");
}

#[test]
fn identical_intent_evaluation_is_deterministic_and_within_scope() {
    let intent = parse_intent_toml(
        r#"version = 1
paths = ["src/auth/**"]
contract_families = []
critical_paths = ["authentication"]
verification = []
"#,
    )
    .expect("intent parses");
    let first = evaluate_intent(
        Some(&intent),
        &["src/auth/login.rs".into()],
        &[],
        &critical_paths(),
        &[],
    );
    let second = evaluate_intent(
        Some(&intent),
        &["src/auth/login.rs".into()],
        &[],
        &critical_paths(),
        &[],
    );

    assert_eq!(first, second);
    assert_eq!(first.status, IntentStatus::WithinScope);
}

#[test]
fn intent_validation_rejects_unknown_ids_and_unsafe_patterns() {
    let intent = parse_intent_toml(
        "version = 1\npaths = [\"../outside/**\"]\nverification = [\"missing\"]\n",
    )
    .expect("syntax is valid before policy validation");
    let mut config = RepoPilotConfig::default();
    config.review.critical_paths = critical_paths();
    let error = validate_intent(&intent, &config).expect_err("unsafe intent must fail");

    assert!(error.to_string().contains("repository-relative"));
}

#[test]
fn intent_json_rejects_unknown_fields_and_oversized_display_metadata() {
    let error = parse_intent_json(&json!({"version": 1, "command": "cargo test"}))
        .expect_err("commands are not part of intent");
    assert!(error.to_string().contains("unknown field"));

    let oversized = "x".repeat(1024);
    let error = parse_intent_json(&json!({"version": 1, "summary": oversized}))
        .expect_err("summary must be bounded");
    assert!(error.to_string().contains("summary"));
}

#[test]
fn intent_file_loader_is_root_confined_and_bounded() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join(".repopilot")).expect("private dir");
    fs::write(
        root.join(".repopilot/intent.toml"),
        "version = 1\npaths = [\"src/**\"]\n",
    )
    .expect("intent file");

    let loaded = load_intent_file(
        root,
        Path::new(".repopilot/intent.toml"),
        &RepoPilotConfig::default(),
    )
    .expect("private intent file loads");
    assert_eq!(loaded.paths, ["src/**"]);

    let outside = root
        .parent()
        .expect("temp parent")
        .join("outside-intent.toml");
    fs::write(&outside, "version = 1\npaths = [\"src/**\"]\n").expect("outside file");
    let error = load_intent_file(
        root,
        Path::new("../outside-intent.toml"),
        &RepoPilotConfig::default(),
    )
    .expect_err("outside intent must be rejected");
    assert!(error.to_string().contains("outside") || error.to_string().contains("root"));

    fs::write(
        root.join(".repopilot/oversized.toml"),
        format!("version = 1\nsummary = \"{}\"\n", "x".repeat(16 * 1024)),
    )
    .expect("oversized file");
    let error = load_intent_file(
        root,
        Path::new(".repopilot/oversized.toml"),
        &RepoPilotConfig::default(),
    )
    .expect_err("oversized intent must be rejected");
    assert!(error.to_string().contains("16") || error.to_string().contains("limit"));
}

#[test]
fn critical_path_policy_rejects_duplicate_and_malformed_rules() {
    let mut config = RepoPilotConfig::default();
    config.review.critical_paths = vec![
        CriticalPathRule {
            name: "auth".to_string(),
            paths: vec!["src/auth/**".to_string()],
        },
        CriticalPathRule {
            name: "auth".to_string(),
            paths: vec!["[".to_string()],
        },
    ];
    let error = validate_critical_paths(&config).expect_err("duplicate policy must fail closed");
    assert!(error.to_string().contains("more than once"));
}

#[test]
fn contract_family_inputs_are_derived_from_canonical_contract_deltas() {
    let delta = ChangeProofContractDelta {
        family: ContractFamily::Dependency,
        change: ContractChangeKind::Changed,
        exporter_path: "Cargo.toml".to_string(),
        consumer_path: "Cargo.toml".to_string(),
        line_start: None,
        line_end: None,
        evidence: "dependency changed".to_string(),
        confidence: Some(ContractConfidence::High),
    };
    let intent = parse_intent_toml("version = 1\ncontract_families = [\"dependency\"]\n")
        .expect("intent parses");
    let result = evaluate_intent(
        Some(&intent),
        &["Cargo.toml".into()],
        &[delta.family],
        &[],
        &[],
    );

    assert_eq!(result.status, IntentStatus::WithinScope);
    assert_eq!(
        result.actual_contract_families,
        vec![ContractFamily::Dependency]
    );
}
