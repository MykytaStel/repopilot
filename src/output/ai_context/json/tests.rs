use super::*;
use crate::findings::types::{Evidence, FindingCategory};
use crate::scan::types::{ScanArtifacts, ScanMetadata, ScanSummary};
use std::path::PathBuf;

fn finding(index: usize, severity: Severity) -> Finding {
    Finding {
        id: format!("rule.example:src/file_{index}.rs:1"),
        rule_id: "rule.example".to_string(),
        title: format!("Example finding {index}"),
        description: "A reasonably long description so each finding carries real \
            weight in the budget accounting and truncation is exercised."
            .to_string(),
        category: FindingCategory::Security,
        severity,
        evidence: vec![Evidence {
            path: PathBuf::from(format!("src/file_{index}.rs")),
            line_start: 1,
            line_end: Some(2),
            snippet: "let secret = \"abcdefghijklmnopqrstuvwxyz0123456789\";".to_string(),
        }],
        ..Default::default()
    }
}

fn summary_with(findings: Vec<Finding>) -> ScanSummary {
    ScanSummary {
        metadata: ScanMetadata {
            root_path: PathBuf::from("repo"),
            ..Default::default()
        },
        metrics: Default::default(),
        artifacts: ScanArtifacts {
            findings,
            ..Default::default()
        },
    }
}

fn parse(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("ai context json is valid")
}

#[test]
fn budget_truncates_low_risk_findings_and_reports_counts() {
    let findings = (0..40)
        .map(|index| finding(index, Severity::High))
        .collect();
    let summary = summary_with(findings);
    // A budget that fits several enriched findings but nowhere near all 40.
    let budget = 600;
    let opts = AiContextRenderOptions {
        focus: None,
        budget_tokens: budget,
        no_header: false,
        no_task: false,
    };

    let value = parse(&render_json(&summary, None, &opts));

    assert_eq!(value["findings_total"], 40);
    let included = value["findings_included"].as_u64().expect("included count");
    assert!(
        (1..40).contains(&included),
        "should include some but not all findings: {included}"
    );
    assert_eq!(
        included + value["findings_omitted"].as_u64().expect("omitted count"),
        40,
        "included + omitted must equal total"
    );
    assert_eq!(value["truncated"], true);
    assert_eq!(
        value["findings"].as_array().expect("findings").len() as u64,
        included
    );

    // approx_tokens is measured from the real (pretty) output and the trim loop
    // keeps it within budget (the only exception is the one-finding signal floor).
    let approx = value["approx_tokens"].as_u64().expect("approx_tokens");
    assert!(
        approx <= budget as u64,
        "approx_tokens {approx} should stay within the {budget}-token budget"
    );
}

#[test]
fn small_repo_is_not_truncated_and_carries_full_evidence() {
    let summary = summary_with(vec![finding(0, Severity::High)]);
    let opts = AiContextRenderOptions {
        focus: None,
        budget_tokens: 4096,
        no_header: false,
        no_task: false,
    };

    let value = parse(&render_json(&summary, None, &opts));

    assert_eq!(value["truncated"], false);
    assert_eq!(value["findings_total"], 1);
    assert_eq!(value["findings_included"], 1);

    let finding = &value["findings"][0];
    assert_eq!(finding["rule_id"], "rule.example");
    assert!(finding["risk"]["score"].is_number(), "risk score present");
    assert!(
        finding["evidence"][0]["snippet"]
            .as_str()
            .expect("snippet")
            .contains("secret"),
        "evidence snippet is included"
    );
    assert!(
        finding["recommendation"].is_string(),
        "recommendation present"
    );
}

#[test]
fn risk_level_exposes_machine_token_and_display_label() {
    let summary = summary_with(vec![finding(0, Severity::High)]);
    let value = parse(&render_json(
        &summary,
        None,
        &AiContextRenderOptions::default(),
    ));

    let level = value["risk"]["level"].as_str().expect("level");
    let label = value["risk"]["label"].as_str().expect("label");
    assert_eq!(
        level,
        level.to_ascii_lowercase(),
        "level is a machine token"
    );
    assert!(
        label.to_uppercase().contains(&level.to_uppercase()),
        "display label `{label}` should carry the machine level `{level}`"
    );
}
