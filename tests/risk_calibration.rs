use repopilot::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use repopilot::risk::{GraphImpact, RiskInputs, assess_finding};
use repopilot::risk::{RiskPriority, RiskSummary};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn clean_small_rust_cli_has_no_high_priority_risk() {
    let temp = tempdir().expect("temp dir");
    write_file(
        temp.path(),
        "src/main.rs",
        r#"
fn main() {
    println!("hello");
}
"#,
    );
    let config = ScanConfig {
        detect_missing_tests: false,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("scan");
    let risk = RiskSummary::from_findings(&summary.artifacts.findings);

    assert_eq!(risk.counts.p0, 0);
    assert_eq!(risk.counts.p1, 0);
}

#[test]
fn repeated_render_module_patterns_are_clustered() {
    let temp = tempdir().expect("temp dir");
    for file in ["a.rs", "b.rs", "c.rs"] {
        write_file(
            temp.path(),
            &format!("src/output/{file}"),
            r#"
pub fn render(value: Option<&str>) -> String {
    value.unwrap().to_string()
}
"#,
        );
    }
    let config = ScanConfig {
        detect_missing_tests: false,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("scan");
    let render_findings =
        findings_for_rule(&summary.artifacts.findings, "language.rust.panic-risk");

    assert_eq!(render_findings.len(), 3);
    assert!(render_findings.iter().all(|finding| {
        finding
            .risk
            .signals
            .iter()
            .any(|signal| signal.id == "cluster.repeated")
    }));
}

#[test]
fn polyglot_backend_prioritizes_secret_over_backlog_marker() {
    let temp = tempdir().expect("temp dir");
    write_file(
        temp.path(),
        "src/config.py",
        r#"API_KEY = "sk_live_123456789abcdef"
"#,
    );
    write_file(
        temp.path(),
        "server/main.go",
        r#"
package main

// TODO: replace demo handler
func main() {}
"#,
    );
    let config = ScanConfig {
        detect_missing_tests: false,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("scan");
    let secret = first_rule(&summary.artifacts.findings, "security.secret-candidate");
    let todo = first_rule(&summary.artifacts.findings, "code-marker.todo");

    assert!(secret.risk.score > todo.risk.score);
    assert!(matches!(
        secret.risk.priority,
        RiskPriority::P0 | RiskPriority::P1
    ));
    assert_eq!(todo.risk.priority, RiskPriority::P3);
}

#[test]
fn react_native_fixture_keeps_framework_findings_explainable_not_critical() {
    let temp = tempdir().expect("temp dir");
    write_file(
        temp.path(),
        "package.json",
        r#"{"dependencies":{"react":"18.2.0","react-native":"0.73.0"}}"#,
    );
    write_file(
        temp.path(),
        "src/App.tsx",
        r#"
import React from "react";
import { Text } from "react-native";

export function App() {
  return <Text style={{ color: "red" }}>Hi</Text>;
}
"#,
    );
    let config = ScanConfig {
        detect_missing_tests: false,
        ..ScanConfig::default()
    };

    let summary = scan_path_with_config(temp.path(), &config).expect("scan");
    let inline_style = first_rule(
        &summary.artifacts.findings,
        "framework.react-native.inline-style",
    );

    assert_eq!(inline_style.risk.formula_version, "risk-v3");
    assert_ne!(inline_style.risk.priority, RiskPriority::P0);
}

#[test]
fn risk_v3_calibration_fixture_matches_snapshot_expectations() {
    let fixture: RiskCalibrationFixture =
        serde_json::from_str(include_str!("fixtures/risk/risk-v3-calibration.json"))
            .expect("risk calibration fixture should parse");

    assert_eq!(fixture.formula_version, "risk-v3");

    for case in fixture.cases {
        let finding = calibration_finding(&case);
        let risk = assess_finding(&finding, None, case.inputs());
        let signal_ids = risk
            .signals
            .iter()
            .map(|signal| signal.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            risk.score, case.expected_score,
            "unexpected score for {} with signals {:?}",
            case.name, signal_ids
        );
        assert_eq!(
            risk.priority, case.expected_priority,
            "unexpected priority for {} with signals {:?}",
            case.name, signal_ids
        );
        for expected_signal in &case.expected_signals {
            assert!(
                signal_ids.contains(&expected_signal.as_str()),
                "{} missing expected risk signal `{}` in {:?}",
                case.name,
                expected_signal,
                signal_ids
            );
        }
    }
}

fn write_file(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write file");
}

fn first_rule<'a>(findings: &'a [Finding], rule_id: &str) -> &'a Finding {
    findings
        .iter()
        .find(|finding| finding.rule_id == rule_id)
        .unwrap_or_else(|| panic!("missing finding for {rule_id}"))
}

fn findings_for_rule<'a>(findings: &'a [Finding], rule_id: &str) -> Vec<&'a Finding> {
    findings
        .iter()
        .filter(|finding| finding.rule_id == rule_id)
        .collect()
}

#[derive(Debug, Deserialize)]
struct RiskCalibrationFixture {
    formula_version: String,
    cases: Vec<RiskCalibrationCase>,
}

#[derive(Debug, Deserialize)]
struct RiskCalibrationCase {
    name: String,
    rule_id: String,
    category: FindingCategory,
    severity: Severity,
    confidence: Confidence,
    #[serde(default)]
    in_diff: bool,
    #[serde(default)]
    blast_radius: bool,
    #[serde(default)]
    cluster_size: usize,
    graph_impact: Option<String>,
    expected_score: u8,
    expected_priority: RiskPriority,
    expected_signals: Vec<String>,
}

impl RiskCalibrationCase {
    fn inputs(&self) -> RiskInputs {
        RiskInputs {
            in_diff: self.in_diff,
            blast_radius: self.blast_radius,
            cluster_size: self.cluster_size,
            graph_impact: self.graph_impact.as_deref().map(|value| match value {
                "hub" => GraphImpact::Hub,
                "dependency" => GraphImpact::Dependency,
                other => panic!("unknown graph impact `{other}`"),
            }),
            ..RiskInputs::default()
        }
    }
}

fn calibration_finding(case: &RiskCalibrationCase) -> Finding {
    Finding {
        id: String::new(),
        rule_id: case.rule_id.clone(),
        title: case.name.clone(),
        description: case.name.clone(),
        recommendation: Finding::recommendation_for_rule_id(&case.rule_id),
        category: case.category.clone(),
        severity: case.severity,
        confidence: case.confidence,
        evidence: vec![Evidence {
            path: PathBuf::from("src/main.rs"),
            line_start: 1,
            line_end: None,
            snippet: String::new(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}
