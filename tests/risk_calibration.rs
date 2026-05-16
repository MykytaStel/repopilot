use repopilot::findings::types::Finding;
use repopilot::risk::{RiskPriority, RiskSummary};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use std::fs;
use std::path::Path;
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
    let risk = RiskSummary::from_findings(&summary.findings);

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
    let render_findings = findings_for_rule(&summary.findings, "language.rust.panic-risk");

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
    let secret = first_rule(&summary.findings, "security.secret-candidate");
    let todo = first_rule(&summary.findings, "code-marker.todo");

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
    let inline_style = first_rule(&summary.findings, "framework.react-native.inline-style");

    assert_eq!(inline_style.risk.formula_version, "risk-v2");
    assert_ne!(inline_style.risk.priority, RiskPriority::P0);
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
